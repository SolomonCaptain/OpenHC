use std::env;
use std::path::PathBuf;

fn main() {
    // 获取项目根目录
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // 计算 libhscir.dll 绝对路径
    let hscir_lib_path = PathBuf::from(&manifest_dir)
        .join("..\\..\\HSCIR\\targets\\Release");

    // 将路径转换为绝对路径并规范化
    let hscir_lib_path = hscir_lib_path.canonicalize().unwrap_or_else(|_| {
        panic!(
            "Cannot find HSCIR library at {}. Please ensure the path exists.",
            hscir_lib_path.display()
        )
    });

    // 告诉 cargo 在哪里查找库
    println!("cargo:rustc-link-search=native={}", hscir_lib_path.display());

    // 指定要链接的库名称（不含前缀和后缀）
    // Windows 上会查找 libhscir.dll 或 hscir.lib
    println!("cargo:rustc-link-lib=dylib=hscir");

    // 重新编译时如果库发生变化
    println!("cargo:rerun-if-changed={}", hscir_lib_path.join("libhscir.dll").display());
}