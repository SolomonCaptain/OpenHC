use anyhow::Result;
use std::fs;
use std::process::Command;

pub fn compile_cuda(source_file: &str, output_file: &str) -> Result<()> {
    // 调用 nvcc
    let status = Command::new("nvcc")
        .args(&[source_file, "-o", output_file, "-arch=sm_61"])
        .status()?;
    if !status.success() { 
        anyhow::bail!("nvcc compilation failed");
    }
    Ok(())
}

pub fn write_cpp_file(content: &str, path: &str) -> Result<()> {
    fs::write(path, content)?;
    Ok(())
}

/// 编译 CPU C++ 代码
/// 
/// 使用 g++ 或 clang++ 编译，支持 OpenMP 并行化
pub fn compile_cpp_host(source_file: &str, output_file: &str) -> Result<()> {
    // 尝试检测可用的编译器
    let compiler = if Command::new("g++").arg("--version").output().is_ok() {
        "g++"
    } else if Command::new("clang++").arg("--version").output().is_ok() {
        "clang++"
    } else if Command::new("cl").arg("/?").output().is_ok() {
        // Windows MSVC
        return compile_with_msvc(source_file, output_file);
    } else {
        anyhow::bail!("No C++ compiler found (g++, clang++, or cl)");
    };

    // 编译选项
    let args = vec![
        source_file.to_string(),
        "-o".to_string(),
        output_file.to_string(),
        "-fopenmp".to_string(),      // OpenMP 支持
        "-O2".to_string(),           // 优化级别
        "-std=c++17".to_string(),    // C++17 标准
        "-pthread".to_string(),      // 线程支持
    ];

    let status = Command::new(compiler)
        .args(&args)
        .status()?;

    if !status.success() {
        anyhow::bail!("{} compilation failed", compiler);
    }

    Ok(())
}

/// 使用 MSVC 编译（Windows）
#[cfg(target_os = "windows")]
fn compile_with_msvc(source_file: &str, output_file: &str) -> Result<()> {
    // MSVC 编译选项
    let args = vec![
        source_file.to_string(),
        format!("/Fe{}", output_file),  // 输出文件名
        "/openmp".to_string(),           // OpenMP 支持
        "/O2".to_string(),               // 优化级别
        "/std:c++17".to_string(),        // C++17 标准
        "/EHsc".to_string(),             // 异常处理
    ];

    let status = Command::new("cl")
        .args(&args)
        .status()?;

    if !status.success() {
        anyhow::bail!("MSVC compilation failed");
    }

    Ok(())
}

/// 非 Windows 平台的 MSVC 编译桩函数
#[cfg(not(target_os = "windows"))]
fn compile_with_msvc(_source_file: &str, _output_file: &str) -> Result<()> {
    anyhow::bail!("MSVC is only available on Windows");
}