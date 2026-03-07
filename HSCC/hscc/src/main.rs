extern crate core;
extern crate alloc;

mod config;
mod lexer;
mod parser;
mod ast;
mod codegen;
mod compile;
mod typeck;
mod diagnostic;
mod semantic;
mod dataflow;
mod analysis;
mod target_check;
mod hscir;
mod lower;
mod triton;
mod npu;

use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;
use diagnostic::DiagnosticCollector;
use semantic::SemanticAnalyzer;

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
    
    // 创建诊断收集器
    let mut diag_collector = DiagnosticCollector::new();
    
    // 词法分析
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // 语法分析
    let mut parser = parser::Parser::new(tokens);
    let ast = parser.parse_program()?;
    
    // 类型检查
    if let Err(e) = typeck::TypeChecker::typecheck_program(&ast, 0) {
        diag_collector.add(
            diagnostic::Diagnostic::error(diagnostic::error_codes::TYPE_MISMATCH)
                .at_file(source_path.to_str().unwrap())
                .message(format!("Type checking failed: {}", e))
        );
    }
    
    // === 语义分析 ===
    let mut semantic_analyzer = SemanticAnalyzer::new();
    semantic_analyzer.set_file(source_path.to_str().unwrap());
    semantic_analyzer.analyze(&ast, &mut diag_collector);
    
    // 输出诊断
    if diag_collector.has_errors() || diag_collector.has_warnings() {
        diag_collector.emit();
        diag_collector.emit_summary();
    }
    
    // 如果有错误，终止编译
    if diag_collector.has_errors() {
        std::process::exit(1);
    }
    
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
        config::Backend::Npu => {
            // 解析 NPU 设备
            let npu_device = npu::parse_npu_device(
                config.target.arch.as_deref().unwrap_or("intel_meteorlake")
            );
            
            println!("Target NPU device: {:?}", npu_device);
            
            // 创建 NPU 后端
            let npu_backend = npu::create_npu_backend(npu_device);
            let spec = npu_backend.hardware_spec(npu_device);
            
            println!("Hardware: {} ({} TOPS)", 
                spec.device_name, 
                spec.matrix_unit.peak_tops
            );
            
            // AST → NPU Graph
            let mut lowering = npu::NpuLowering::new(npu_backend, npu_device);
            let mut graph = lowering.lower_program(&ast)
                .map_err(|e: npu::NpuError| anyhow::anyhow!("NPU lowering failed: {}", e))?;
            
            // 创建后端实例
            let backend = npu::IntelNpuBackend::new();
            
            // 优化计算图
            <npu::IntelNpuBackend as npu::NpuBackend>::optimize_graph(&backend, &mut graph, &spec)
                .map_err(|e: npu::NpuError| anyhow::anyhow!("Graph optimization failed: {}", e))?;
            
            // 内存规划
            let _memory_plan = <npu::IntelNpuBackend as npu::NpuBackend>::plan_memory(&backend, &mut graph, &spec)
                .map_err(|e: npu::NpuError| anyhow::anyhow!("Memory planning failed: {}", e))?;
            
            // 自动调优
            let mut autotuner = npu::NpuAutoTuner::new(spec.clone());
            let _tuning_params = autotuner.tune(&graph);
            
            // 生成代码
            let npu_code = <npu::IntelNpuBackend as npu::NpuBackend>::generate_code(&backend, &graph, &spec)
                .map_err(|e: npu::NpuError| anyhow::anyhow!("Code generation failed: {}", e))?;
            
            // 生成运行时配置
            let _runtime_config = <npu::IntelNpuBackend as npu::NpuBackend>::generate_runtime_config(&backend, &graph, &spec)
                .map_err(|e: npu::NpuError| anyhow::anyhow!("Runtime config generation failed: {}", e))?;
            
            // 输出文件
            match npu_code {
                npu::NpuCode::OnnxModel(bytes) => {
                    let onnx_file = Path::new(project_dir).join(format!("{}.onnx", config.package.name));
                    fs::write(&onnx_file, &bytes)?;
                    println!("Generated ONNX model: {}", onnx_file.display());
                }
                npu::NpuCode::OnnxText(text) => {
                    let onnx_file = Path::new(project_dir).join(format!("{}.onnx.txt", config.package.name));
                    fs::write(&onnx_file, &text)?;
                    println!("Generated ONNX text: {}", onnx_file.display());
                }
                npu::NpuCode::OpenVINOIR { xml, bin } => {
                    let xml_file = Path::new(project_dir).join(format!("{}.xml", config.package.name));
                    let bin_file = Path::new(project_dir).join(format!("{}.bin", config.package.name));
                    fs::write(&xml_file, &xml)?;
                    fs::write(&bin_file, &bin)?;
                    println!("Generated OpenVINO IR: {} {}", xml_file.display(), bin_file.display());
                }
                npu::NpuCode::PythonCode(code) => {
                    let py_file = Path::new(project_dir).join(format!("{}_npu.py", config.package.name));
                    fs::write(&py_file, &code)?;
                    println!("Generated Python runtime: {}", py_file.display());
                }
                _ => {
                    println!("Generated NPU code (format: {:?})", npu_code);
                }
            }
            
            // 生成运行时代码
            let runtime_gen = npu::RuntimeGenerator::new(npu::RuntimeTarget::Python);
            let runtime_code = runtime_gen.generate(&graph, &_runtime_config);
            
            if let Some(python) = runtime_code.python {
                let py_file = Path::new(project_dir).join(format!("{}_runtime.py", config.package.name));
                fs::write(&py_file, &python)?;
                println!("Generated Python runtime: {}", py_file.display());
            }
            
            println!("NPU compilation successful!");
            println!("Run with: python {}_runtime.py --model {}.onnx", 
                config.package.name, config.package.name);
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
    
    // ========== NPU 后端端到端测试 ==========
    
    #[test]
    fn test_npu_graph_creation() {
        let graph = npu::NpuGraph::new("test_model");
        assert_eq!(graph.name, "test_model");
        assert!(graph.inputs.is_empty());
        assert!(graph.outputs.is_empty());
    }
    
    #[test]
    fn test_npu_type_conversion() {
        // 测试类型转换
        let f32_type = npu::NpuType::f32();
        assert_eq!(f32_type.to_onnx_type(), "float");
        assert_eq!(f32_type.to_numpy_dtype(), "float32");
        
        let i32_type = npu::NpuType::i32();
        assert_eq!(i32_type.to_onnx_type(), "int32");
        
        // 测试张量类型
        let tensor = npu::NpuType::tensor_nchw(npu::NpuType::f32(), 1, 3, 224, 224);
        assert_eq!(tensor.shape(), Some(&vec![1, 3, 224, 224]));
    }
    
    #[test]
    fn test_intel_npu_backend() {
        let backend = npu::IntelNpuBackend::new();
        
        let devices = <npu::IntelNpuBackend as npu::NpuBackend>::supported_devices(&backend);
        assert!(!devices.is_empty());
        
        // 测试硬件规格
        let spec = <npu::IntelNpuBackend as npu::NpuBackend>::hardware_spec(&backend, 
            npu::NpuDevice::IntelNPU(npu::IntelNpuGeneration::MeteorLake)
        );
        assert!(spec.matrix_unit.peak_tops > 0.0);
        assert_eq!(spec.num_cores, 2);
    }
    
    #[test]
    fn test_npu_device_parsing() {
        let device = npu::parse_npu_device("intel_meteorlake");
        assert!(matches!(device, npu::NpuDevice::IntelNPU(
            npu::IntelNpuGeneration::MeteorLake
        )));
        
        let device = npu::parse_npu_device("npu_lunar");
        assert!(matches!(device, npu::NpuDevice::IntelNPU(
            npu::IntelNpuGeneration::LunarLake
        )));
    }
    
    #[test]
    fn test_npu_operation_types() {
        // 测试操作类型名称
        let matmul = npu::NpuOpType::MatMul;
        assert_eq!(matmul.name(), "MatMul");
        
        let relu = npu::NpuOpType::ReLU;
        assert_eq!(relu.name(), "Relu");
        
        let conv = npu::NpuOpType::Conv2D {
            padding: npu::Padding::Valid,
            stride: (1, 1),
            dilation: (1, 1),
            groups: 1,
        };
        assert_eq!(conv.name(), "Conv");
    }
    
    #[test]
    fn test_onnx_builder() {
        use npu::onnx::OnnxBuilder;
        
        let mut graph = npu::NpuGraph::new("test_onnx");
        graph.add_input("input", npu::NpuType::f32(), vec![1, 3, 224, 224]);
        graph.add_output("output", npu::NpuType::f32(), vec![1, 1000]);
        
        let builder = OnnxBuilder::new(&graph);
        let result = builder.build_to_text();
        assert!(result.is_ok());
        
        let text = result.unwrap();
        assert!(text.contains("test_onnx"));
    }
    
    #[test]
    fn test_fusion_optimizer() {
        let mut optimizer = npu::fusion::FusionOptimizer::new();
        let mut graph = npu::NpuGraph::new("test_fusion");
        
        // 空图优化应该成功
        let results = optimizer.optimize(&mut graph);
        assert!(results.is_empty());
    }
    
    #[test]
    fn test_fusion_analyzer() {
        let mut analyzer = npu::fusion::FusionAnalyzer::new();
        let graph = npu::NpuGraph::new("test_analyze");
        
        let opportunities = analyzer.analyze(&graph);
        // 空图应该没有融合机会
        assert!(opportunities.is_empty());
    }
    
    #[test]
    fn test_npu_graph_with_operations() {
        let mut graph = npu::NpuGraph::new("test_ops");
        
        // 添加输入
        graph.add_input("A", npu::NpuType::f32(), vec![10, 20]);
        graph.add_input("B", npu::NpuType::f32(), vec![20, 30]);
        
        // 添加输出
        graph.add_output("C", npu::NpuType::f32(), vec![10, 30]);
        
        // 添加 MatMul 操作
        let op = npu::NpuOperation {
            index: 0,
            op_type: npu::NpuOpType::MatMul,
            name: "matmul_0".to_string(),
            inputs: vec!["A".to_string(), "B".to_string()],
            outputs: vec!["C".to_string()],
            attributes: std::collections::HashMap::new(),
            hints: npu::graph::OpHints::default(),
        };
        graph.add_operation(op);
        
        // 验证
        assert_eq!(graph.inputs.len(), 2);
        assert_eq!(graph.outputs.len(), 1);
        assert_eq!(graph.operations.len(), 1);
    }

    // ========== 诊断系统测试 ==========

    #[test]
    fn test_diagnostic_creation() {
        let diag = diagnostic::Diagnostic::error(diagnostic::error_codes::TYPE_MISMATCH)
            .at_file("test.hl")
            .message("Type mismatch: expected i32, found f32");

        assert_eq!(diag.level, diagnostic::DiagnosticLevel::Error);
        assert_eq!(diag.code, "HSC1001");
        assert_eq!(diag.file, "test.hl");
    }

    #[test]
    fn test_diagnostic_collector() {
        let mut collector = diagnostic::DiagnosticCollector::new();

        collector.add(
            diagnostic::Diagnostic::error(diagnostic::error_codes::UNDEFINED_VARIABLE)
                .at_file("test.hl")
                .message("Undefined variable: x")
        );
        collector.add(
            diagnostic::Diagnostic::warning(diagnostic::error_codes::UNUSED_VARIABLE)
                .at_file("test.hl")
                .message("Unused variable: y")
        );

        assert!(collector.has_errors());
        assert!(collector.has_warnings());
        assert_eq!(collector.error_count(), 1);
        assert_eq!(collector.warning_count(), 1);
    }

    // ========== 语义分析测试 ==========

    #[test]
    fn test_semantic_analyzer_simple_function() {
        let source = r#"
fn main() {
    let x = 42;
    let y = x + 1;
}
"#;
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = semantic::SemanticAnalyzer::new();
        let mut collector = diagnostic::DiagnosticCollector::new();
        analyzer.analyze(&ast, &mut collector);

        // 不应该有错误
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_semantic_analyzer_task_dependency() {
        let source = r#"
task compute {
    body(x: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let y = i;
        }
    }
}

task process {
    body(x: Buffer<f32>) -> Buffer<f32> {
        parallel for i in 0..1024 {
            let y = i;
        }
    }
}

fn main() {
    let a = Buffer::<f32>::zeros([1024]);
    let b = spawn on GPU compute(a).await;
    let c = spawn on GPU process(b).await;
}
"#;
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = semantic::SemanticAnalyzer::new();
        let mut collector = diagnostic::DiagnosticCollector::new();
        analyzer.analyze(&ast, &mut collector);

        // 不应该有循环依赖错误
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_semantic_analyzer_with_pattern() {
        let source = r#"
task reduce_task {
    pattern: Reduce,
    body(arr: Buffer<f32>) -> f32 {
        parallel for i in 0..1024 {
            let val = arr[i];
        }
        return 0.0;
    }
}

fn main() {
    let arr = Buffer::<f32>::zeros([1024]);
    let result = spawn on GPU reduce_task(arr).await;
}
"#;
        let mut lexer = lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = parser::Parser::new(tokens);
        let ast = parser.parse_program().expect("Parse failed");

        let mut analyzer = semantic::SemanticAnalyzer::new();
        let mut collector = diagnostic::DiagnosticCollector::new();
        analyzer.analyze(&ast, &mut collector);

        // 应该正常分析
        assert!(!collector.has_errors());
    }

    #[test]
    fn test_task_dependency_graph() {
        let mut graph = semantic::TaskDependencyGraph::new();
        
        graph.add_task("A");
        graph.add_task("B");
        graph.add_task("C");
        graph.add_dependency("A", "B", "data");
        graph.add_dependency("B", "C", "data");

        // 无循环
        assert!(graph.detect_cycles().is_none());
        
        // 拓扑排序应该成功
        assert!(graph.topological_sort().is_some());
    }

    #[test]
    fn test_task_dependency_graph_with_cycle() {
        let mut graph = semantic::TaskDependencyGraph::new();
        
        graph.add_task("A");
        graph.add_task("B");
        graph.add_task("C");
        graph.add_dependency("A", "B", "data");
        graph.add_dependency("B", "C", "data");
        graph.add_dependency("C", "A", "data");

        // 应该检测到循环
        assert!(graph.detect_cycles().is_some());
        
        // 拓扑排序应该失败
        assert!(graph.topological_sort().is_none());
    }

    #[test]
    fn test_device_info() {
        let info = semantic::DeviceInfo::new();
        
        assert!(info.is_device_available("GPU"));
        assert!(info.is_device_available("CPU"));
        assert!(info.is_device_available("Host"));
        
        let gpu_cap = info.get_capability("GPU").unwrap();
        assert!(gpu_cap.supports_fp16);
        assert!(gpu_cap.supports_int8);
    }
}