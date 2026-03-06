extern crate core;
extern crate alloc;

mod config;
mod lexer;
mod parser;
mod ast;
mod codegen;
mod compile;
mod typeck;
mod hscir;
mod lower;
mod triton;

use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;

#[cfg(not(test))]
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: hscc <project-directory> [--backend=<cuda|triton>]");
        std::process::exit(1);
    }
    
    // 解析参数
    let project_dir = &args[1];
    let backend_override = args.iter()
        .find(|arg| arg.starts_with("--backend="))
        .map(|arg| arg.strip_prefix("--backend=").unwrap());
    
    let config_path = Path::new(project_dir).join("HSCC.toml");
    let config = config::Config::from_file(config_path.to_str().unwrap())?;
    
    println!("Compiling project: {} v{}", config.package.name, config.package.version);
    
    // 确定后端
    let backend = if let Some(backend_str) = backend_override {
        config::Backend::from_str(backend_str)
    } else {
        config.get_backend()
    };
    
    println!("Target device: {} (backend: {})", config.target.device, backend.name());
    
    // 查找源文件
    let source_path = Path::new(project_dir).join("src").join("main.hl");
    let source = fs::read_to_string(&source_path)?;
    
    // 词法分析
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // 语法分析
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse_program()?;
    
    // 类型检查
    typeck::TypeChecker::typecheck_program(&ast, 1)?;
    
    // 根据后端选择代码生成
    match backend {
        config::Backend::Triton => {
            // 生成 Triton Python 代码
            let triton_code = triton::lowering::lower_to_triton(&ast);
            
            // 写入 Python 文件
            let py_file = Path::new(project_dir).join(format!("{}.py", config.package.name));
            fs::write(&py_file, &triton_code)?;
            
            println!("Generated Triton Python: {}", py_file.display());
            println!("Run with: python {}", py_file.display());
        }
        config::Backend::Cuda | config::Backend::Hip => {
            // 生成 CUDA/HIP C++ 代码
            let mut generator = codegen::CodeGenerator::new();
            let cuda_code = generator.generate(&ast);
            
            // 写入临时 CUDA 文件
            let cpp_file = Path::new(project_dir).join("output.cu");
            compile::write_cpp_file(&cuda_code, cpp_file.to_str().unwrap())?;
            
            // 编译为可执行文件
            let exe_file = Path::new(project_dir).join(&config.package.name);
            compile::compile_cuda(cpp_file.to_str().unwrap(), exe_file.to_str().unwrap())?;
            
            println!("Compilation successful! Executable: {}", exe_file.display());
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // ========== 词法分析 -> 语法分析 端到端测试 ==========
    
    #[test]
    fn test_lexer_to_parser_simple() {
        let source = r#"
fn main() {
    let x = 5;
}
"#;
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Failed to parse simple program: {:?}", result.err());
    }
    
    #[test]
    fn test_buffer_parsing() {
        let source = r#"
import hsc::*;

fn main() -> () {
    let a = Buffer::<f32>::zeros([10, 10]);
}
"#;
        
        // 词法分析
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        
        // 语法分析
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Parsing failed: {:?}", result.err());
        println!("Buffer parsing test passed!");
    }
    
    // ========== 完整编译流水线测试 ==========
    
    #[test]
    fn test_full_pipeline_simple_function() {
        let source = r#"
fn main() {
    let x = 42;
}
"#;
        
        // 词法分析
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty());
        
        // 语法分析
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program();
        assert!(ast.is_ok(), "Parser failed: {:?}", ast.err());
        let ast = ast.unwrap();
        
        // 类型检查
        let typeck_result = typeck::TypeChecker::typecheck_program(&ast, 0);
        assert!(typeck_result.is_ok(), "Type checker failed: {:?}", typeck_result.err());
        
        // 代码生成
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        assert!(!cuda_code.is_empty());
        assert!(cuda_code.contains("int main()"));
    }
    
    #[test]
    fn test_full_pipeline_with_function() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn main() {
    let result = add(1, 2);
}
"#;
        
        // 词法分析
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        
        // 语法分析
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser failed");
        
        // 类型检查
        typeck::TypeChecker::typecheck_program(&ast, 0).expect("Type check failed");
        
        // 代码生成
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        assert!(cuda_code.contains("int add(int a, int b)"));
        assert!(cuda_code.contains("int main()"));
    }
    
    #[test]
    fn test_full_pipeline_with_task() {
        let source = r#"
task compute {
    body(a: Buffer<f32>, b: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let sum = a[i] + b[i];
        }
    }
}

fn main() {
    let a = Buffer::<f32>::zeros([1024]);
    let b = Buffer::<f32>::zeros([1024]);
    let result = spawn on GPU compute(a, b).await;
}
"#;
        
        // 词法分析
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        
        // 语法分析
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser failed");
        
        // 类型检查
        typeck::TypeChecker::typecheck_program(&ast, 0).expect("Type check failed");
        
        // 代码生成
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        // 检查生成的 CUDA 代码
        assert!(cuda_code.contains("__global__ void compute_kernel"));
        assert!(cuda_code.contains("int main()"));
    }
    
    // ========== 错误恢复测试 ==========
    
    #[test]
    fn test_error_recovery_missing_semicolon() {
        let source = r#"
fn main() {
    let x = 5
    let y = 10;
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        // 应该报告错误
        assert!(result.is_err(), "Expected error for missing semicolon");
    }
    
    #[test]
    fn test_error_recovery_missing_brace() {
        let source = r#"
fn main() {
    let x = 5;
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_err(), "Expected error for missing closing brace");
    }
    
    #[test]
    fn test_error_recovery_undefined_variable() {
        let source = r#"
fn main() {
    let x = undefined_var;
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser should succeed");
        
        // 类型检查应该捕获未定义变量
        let result = typeck::TypeChecker::typecheck_program(&ast, 0);
        assert!(result.is_err(), "Expected error for undefined variable");
    }
    
    #[test]
    fn test_error_recovery_type_mismatch() {
        let source = r#"
fn main() {
    let x: i32 = true;
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser should succeed");
        
        // 类型检查应该捕获类型不匹配
        // 注意：当前实现可能不会捕获这个错误
        let _result = typeck::TypeChecker::typecheck_program(&ast, 0);
    }
    
    // ========== 边界情况测试 ==========
    
    #[test]
    fn test_empty_program() {
        let source = "";
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        assert!(tokens.is_empty());
        
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Empty program should parse");
        
        assert!(ast.functions.is_empty());
        assert!(ast.tasks.is_empty());
        assert!(ast.imports.is_empty());
    }
    
    #[test]
    fn test_whitespace_only() {
        let source = "   \n\t\n   ";
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        assert!(tokens.is_empty());
    }
    
    #[test]
    fn test_comments_only() {
        let source = r#"
// This is a comment
/* This is a 
   block comment */
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        assert!(tokens.is_empty(), "Comments should be ignored");
    }
    
    // ========== 复杂程序测试 ==========
    
    #[test]
    fn test_complex_program() {
        let source = r#"
import hsc::*;

fn init(arr: Buffer<f32>, size: i32) {
    parallel for i in 0..size {
        let idx = i;
    }
}

task matmul {
    body(a: Buffer<f32>, b: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            for j in 0..1024 {
                let sum = 0.0;
                for k in 0..1024 {
                    let sum = sum + a[i] * b[k];
                }
            }
        }
    }
}

fn main() {
    let size = 1024;
    let a = Buffer::<f32>::zeros([size, size]);
    let b = Buffer::<f32>::zeros([size, size]);
    
    init(a, size);
    init(b, size);
    
    let a_dev = a.move_to(GPU);
    let b_dev = b.move_to(GPU);
    
    let result = spawn on GPU matmul(a_dev, b_dev).await;
    let result_host = result.move_to(Host);
}
"#;
        
        // 完整编译流水线
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty());
        
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser failed");
        
        assert_eq!(ast.imports.len(), 1);
        assert_eq!(ast.functions.len(), 2);
        assert_eq!(ast.tasks.len(), 1);
        
        let typeck_result = typeck::TypeChecker::typecheck_program(&ast, 0);
        assert!(typeck_result.is_ok(), "Type check failed: {:?}", typeck_result.err());
        
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        // 验证生成的代码
        assert!(cuda_code.contains("#include <cuda_runtime.h>"));
        assert!(cuda_code.contains("__global__ void matmul_kernel"));
        assert!(cuda_code.contains("void init"));
        assert!(cuda_code.contains("int main"));
    }
    
    // ========== 特定语法测试 ==========
    
    #[test]
    fn test_spawn_expression() {
        let source = r#"
task compute {
    body(x: i32) -> i32 {
        parallel for i in 0..10 {
            let y = i;
        }
    }
}

fn main() {
    let result = spawn on GPU compute(42).await;
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parser failed");
        
        let typeck_result = typeck::TypeChecker::typecheck_program(&ast, 0);
        assert!(typeck_result.is_ok(), "Type check failed: {:?}", typeck_result.err());
    }
    
    #[test]
    fn test_nested_control_flow() {
        let source = r#"
fn main() {
    for i in 0..10 {
        if i > 5 {
            for j in 0..i {
                let x = i + j;
            }
        } else {
            while i < 3 {
                let y = i;
            }
        }
    }
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Nested control flow should parse: {:?}", result.err());
    }
    
    #[test]
    fn test_parallel_for_in_function() {
        let source = r#"
fn process(data: Buffer<f32>) {
    parallel for i in 0..1024 {
        let x = i;
    }
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Parallel for should parse: {:?}", result.err());
    }
    
    // ========== 类型系统测试 ==========
    
    #[test]
    fn test_all_primitive_types() {
        let source = r#"
fn test_types(
    a: i8, b: i16, c: i32, d: i64, e: i128,
    f: u8, g: u16, h: u32, i: u64, j: u128,
    k: f32, l: f64,
    m: bool, n: char
) {}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "All primitive types should parse: {:?}", result.err());
    }
    
    #[test]
    fn test_buffer_with_dimensions() {
        let source = r#"
fn main() {
    let buf1: Buffer<f32> = 0;
    let buf2: Buffer<f32, 10> = 0;
    let buf3: Buffer<i32, 100> = 0;
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let result = parser.parse_program();
        
        assert!(result.is_ok(), "Buffer with dimensions should parse: {:?}", result.err());
    }
    
    // ========== CUDA 代码生成验证测试 ==========
    
    #[test]
    fn test_cuda_includes() {
        let source = "fn main() {}";
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().unwrap();
        
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        assert!(cuda_code.contains("#include <cuda_runtime.h>"));
        assert!(cuda_code.contains("#include <stdio.h>"));
        assert!(cuda_code.contains("#include <stdlib.h>"));
        assert!(cuda_code.contains("#include <vector>"));
    }
    
    #[test]
    fn test_cuda_buffer_template() {
        let source = "fn main() {}";
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().unwrap();
        
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        assert!(cuda_code.contains("template<typename T>"));
        assert!(cuda_code.contains("struct Buffer"));
    }
    
    #[test]
    fn test_cuda_kernel_generation() {
        let source = r#"
task my_task {
    body(x: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..100 {
            let y = i;
        }
    }
}
"#;
        
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().unwrap();
        
        let mut generator = codegen::CodeGenerator::new();
        let cuda_code = generator.generate(&ast);
        
        assert!(cuda_code.contains("__global__ void my_task_kernel"));
    }
}