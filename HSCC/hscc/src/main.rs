extern crate core;
extern crate alloc;

mod config;
mod lexer;
mod parser;
mod ast;
mod codegen;
mod compile;
mod typeck;

use anyhow::Result;
use std::env;
use std::fs;
use std::path::Path;

#[cfg(not(test))]
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: hscc <project-directory>");
        std::process::exit(1);
    }
    let project_dir = &args[1];
    let config_path = Path::new(project_dir).join("HSCC.toml");
    let config = config::Config::from_file(config_path.to_str().unwrap())?;
    println!("Compiling project: {} v{}", config.package.name, config.package.version);
    println!("Target device: {}", config.target.device);
    
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
    
    // 代码生成
    let mut generator = codegen::CodeGenerator::new();
    let cuda_code = generator.generate(&ast);
    
    // 写入临时 CUDA 文件
    let cpp_file = Path::new(project_dir).join("output.cu");
    compile::write_cpp_file(&cuda_code, cpp_file.to_str().unwrap())?;
    
    // 编译为可执行文件
    let exe_file = Path::new(project_dir).join(&config.package.name);
    compile::compile_cuda(cpp_file.to_str().unwrap(), exe_file.to_str().unwrap())?;
    
    println!("Compilation successful! Executable: {}", exe_file.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
}