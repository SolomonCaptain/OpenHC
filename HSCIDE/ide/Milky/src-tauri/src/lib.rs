// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 从项目根目录加载 .env.local 文件
    // 开发时 .env.local 位于项目根目录（src-tauri 的父目录）
    let env_path = std::path::PathBuf::from("..").join(".env.local");
    if env_path.exists() {
        match dotenvy::from_path(&env_path) {
            Ok(_) => {
                println!("[Clerk] Successfully loaded .env.local from project root");
            }
            Err(e) => {
                eprintln!("[Clerk] Warning: Failed to load .env.local: {}", e);
            }
        }
    } else {
        eprintln!("[Clerk] Warning: .env.local not found at {:?}", env_path);
    }

    // 从环境变量获取 Clerk Publishable Key
    // 优先读取 VITE_CLERK_PUBLISHABLE_KEY（与前端共享同一变量）
    let clerk_publishable_key = std::env::var("VITE_CLERK_PUBLISHABLE_KEY")
        .or_else(|_| std::env::var("CLERK_PUBLISHABLE_KEY"))
        .unwrap_or_else(|_| {
            eprintln!("[Clerk] Warning: VITE_CLERK_PUBLISHABLE_KEY not set, using placeholder");
            "pk_test_placeholder".to_string()
        });

    println!("[Clerk] Using publishable key: {}...", &clerk_publishable_key[..20.min(clerk_publishable_key.len())]);

    tauri::Builder::default()
        // 必需：用于请求路由
        .plugin(tauri_plugin_http::init())
        // 可选：用于持久化认证状态
        .plugin(tauri_plugin_store::Builder::new().build())
        // Clerk 认证插件
        .plugin(
            tauri_plugin_clerk::ClerkPluginBuilder::new()
                .publishable_key(clerk_publishable_key)
                .with_tauri_store()
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}