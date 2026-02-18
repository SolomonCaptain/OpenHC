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