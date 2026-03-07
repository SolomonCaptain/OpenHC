//! CLI tool for plugin management.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use hsc_plugin_manager::{PluginManager, LoadOptions, PluginCategory};

#[derive(Parser)]
#[command(name = "hsc-plugin")]
#[command(about = "OpenHC Plugin Management CLI", long_about = None)]
struct Cli {
    /// Plugin search directory
    #[arg(short, long, default_value = "./plugins")]
    plugin_dir: PathBuf,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available plugins
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        
        /// Show only loaded plugins
        #[arg(short, long)]
        loaded: bool,
    },
    
    /// Load a plugin
    Load {
        /// Plugin name or path
        plugin: String,
        
        /// Initialize after loading
        #[arg(short, long)]
        init: bool,
    },
    
    /// Unload a plugin
    Unload {
        /// Plugin name
        plugin: String,
    },
    
    /// Show plugin information
    Info {
        /// Plugin name
        plugin: String,
    },
    
    /// Validate plugin manifest
    Validate {
        /// Path to plugin directory or manifest file
        path: PathBuf,
    },
    
    /// Create a new plugin template
    New {
        /// Plugin name
        name: String,
        
        /// Plugin category
        #[arg(short, long, default_value = "utility")]
        category: String,
        
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    
    /// Check dependencies
    Deps {
        /// Plugin name
        plugin: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let manager = PluginManager::new();
    
    match cli.command {
        Commands::List { category, loaded } => {
            list_plugins(&manager, &cli.plugin_dir, category, loaded)?;
        }
        
        Commands::Load { plugin, init } => {
            load_plugin(&manager, &plugin, init)?;
        }
        
        Commands::Unload { plugin } => {
            unload_plugin(&manager, &plugin)?;
        }
        
        Commands::Info { plugin } => {
            show_info(&manager, &plugin)?;
        }
        
        Commands::Validate { path } => {
            validate_plugin(&path)?;
        }
        
        Commands::New { name, category, output } => {
            create_plugin_template(&name, &category, &output)?;
        }
        
        Commands::Deps { plugin } => {
            check_deps(&manager, &plugin)?;
        }
    }
    
    Ok(())
}

fn list_plugins(
    manager: &PluginManager,
    plugin_dir: &PathBuf,
    category: Option<String>,
    loaded_only: bool,
) -> anyhow::Result<()> {
    let options = LoadOptions {
        search_paths: vec![plugin_dir.clone()],
        ..Default::default()
    };
    
    let discovered = manager.discover(&options)?;
    
    println!("Discovered plugins:");
    println!("{:-<60}", "");
    
    for path in discovered {
        let manifest_path = path.join("plugin.toml");
        if let Ok(manifest) = hsc_plugin_manager::manifest::PluginManifest::from_file(&manifest_path) {
            // Filter by category
            if let Some(ref cat) = category {
                if manifest.plugin.category != *cat {
                    continue;
                }
            }
            
            // Filter by loaded status
            if loaded_only && !manager.is_loaded(&manifest.plugin.name) {
                continue;
            }
            
            let status = if manager.is_loaded(&manifest.plugin.name) {
                "[LOADED]"
            } else {
                "        "
            };
            
            println!(
                "{} {} v{} - {}",
                status,
                manifest.plugin.name,
                manifest.plugin.version,
                manifest.plugin.description
            );
            
            if !manifest.dependencies.is_empty() {
                println!("    Dependencies: {}", 
                    manifest.dependencies.iter()
                        .map(|d| d.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    }
    
    println!();
    println!("Loaded plugins: {:?}", manager.list_loaded());
    
    Ok(())
}

fn load_plugin(manager: &PluginManager, plugin: &str, init: bool) -> anyhow::Result<()> {
    let path = PathBuf::from(plugin);
    
    if path.is_dir() {
        println!("Loading plugin from directory: {}", plugin);
        let handle = manager.load_from_dir(&path)?;
        println!("Loaded: {} v{}", handle.name(), handle.info.version);
        
        if init {
            println!("Initializing...");
            manager.initialize(handle.name())?;
            println!("Initialized successfully.");
        }
    } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
        let default_path = PathBuf::from(".");
        let dir = path.parent().unwrap_or(&default_path);
        let manifest = hsc_plugin_manager::manifest::PluginManifest::from_file(&path)?;
        let lib_path = dir.join(manifest.library_name());
        
        println!("Loading plugin: {}", manifest.plugin.name);
        let handle = manager.load(&lib_path, &manifest)?;
        println!("Loaded: {} v{}", handle.name(), handle.info.version);
    } else {
        anyhow::bail!("Invalid plugin path: must be a directory or manifest file");
    }
    
    Ok(())
}

fn unload_plugin(manager: &PluginManager, plugin: &str) -> anyhow::Result<()> {
    println!("Unloading plugin: {}", plugin);
    manager.unload(plugin)?;
    println!("Unloaded successfully.");
    Ok(())
}

fn show_info(manager: &PluginManager, plugin: &str) -> anyhow::Result<()> {
    let registry = manager.registry();
    
    if let Some(info) = registry.get(plugin) {
        println!("Plugin: {}", info.name);
        println!("{:-<40}", "");
        println!("Version:     {}", info.version);
        println!("Category:    {}", info.category);
        println!("Description: {}", info.description);
        println!("Author:      {}", info.author);
        println!("License:     {}", info.license);
        println!("Library:     {}", info.library_path);
        println!("Loaded:      {}", info.is_loaded);
        
        if !info.operations.is_empty() {
            println!("\nOperations:");
            for op in &info.operations {
                println!("  - {}", op);
            }
        }
        
        if !info.types.is_empty() {
            println!("\nTypes:");
            for t in &info.types {
                println!("  - {}", t);
            }
        }
        
        if !info.dependencies.is_empty() {
            println!("\nDependencies:");
            for dep in &info.dependencies {
                println!("  - {}", dep);
            }
        }
    } else {
        println!("Plugin '{}' not found.", plugin);
    }
    
    Ok(())
}

fn validate_plugin(path: &PathBuf) -> anyhow::Result<()> {
    let manifest_path = if path.is_dir() {
        path.join("plugin.toml")
    } else {
        path.clone()
    };
    
    println!("Validating: {}", manifest_path.display());
    
    match hsc_plugin_manager::manifest::PluginManifest::from_file(&manifest_path) {
        Ok(manifest) => {
            println!("Valid plugin manifest!");
            println!("  Name: {}", manifest.plugin.name);
            println!("  Version: {}", manifest.plugin.version);
            println!("  Category: {}", manifest.plugin.category);
            Ok(())
        }
        Err(e) => {
            println!("Validation failed: {}", e);
            Err(e.into())
        }
    }
}

fn create_plugin_template(name: &str, category: &str, output: &PathBuf) -> anyhow::Result<()> {
    let plugin_dir = output.join(name.replace('.', "_"));
    std::fs::create_dir_all(&plugin_dir)?;
    
    // Create manifest
    let manifest = format!(
        r#"[plugin]
name = "{}"
version = "0.1.0"
description = "A new {} plugin"
author = "Your Name"
license = "Apache-2.0"
category = "{}"

[dependencies]

[resources]
gpu_memory_mb = 256
max_threads = 4

[extensions]
operations = []
types = []
"#,
        name, category, category
    );
    
    std::fs::write(plugin_dir.join("plugin.toml"), manifest)?;
    
    // Create source template
    let src_dir = plugin_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;
    
    let main_src = r#"// Plugin implementation
// See plugin_api.h for interface details

#include <plugin_api.h>
#include <string.h>

// Plugin info
static const char* PLUGIN_NAME = "PLUGIN_NAME";
static const char* PLUGIN_VERSION = "0.1.0";
static const char* PLUGIN_DESC = "Plugin description";

// Exported functions
const HscPluginInfo* plugin_get_info(void) {
    static HscPluginInfo info = {0};
    if (info.name == NULL) {
        info.name = PLUGIN_NAME;
        info.version = PLUGIN_VERSION;
        info.description = PLUGIN_DESC;
        info.author = "Your Name";
        info.license = "Apache-2.0";
        info.category = HSC_CATEGORY_UTILITY;
    }
    return &info;
}

HscErrorCode plugin_initialize(HscPluginContext* ctx, const HscHostServices* services) {
    return HSC_SUCCESS;
}

HscErrorCode plugin_create_instance(const char* config, HscPluginInstance** instance) {
    // Create your plugin instance here
    return HSC_SUCCESS;
}

HscErrorCode plugin_execute(
    HscPluginInstance* instance,
    const char* operation,
    const HscValue* const* inputs,
    uint32_t num_inputs,
    HscValue** outputs,
    uint32_t num_outputs
) {
    return HSC_ERROR_OPERATION_NOT_SUPPORTED;
}

void plugin_destroy_instance(HscPluginInstance* instance) {
    // Cleanup instance
}

HscErrorCode plugin_configure(HscPluginInstance* instance, const char* key, const char* value) {
    return HSC_SUCCESS;
}

HscErrorCode plugin_query(HscPluginInstance* instance, const char* query, char* result, size_t size) {
    return HSC_SUCCESS;
}

HscErrorCode plugin_shutdown(void) {
    return HSC_SUCCESS;
}

// Export entry point
HSC_EXPORT_PLUGIN(
    plugin_get_info,
    plugin_initialize,
    plugin_create_instance,
    plugin_execute,
    plugin_destroy_instance,
    plugin_configure,
    plugin_query,
    plugin_shutdown
)
"#;
    
    std::fs::write(src_dir.join("main.c"), main_src)?;
    
    // Create CMakeLists.txt
    let cmake = format!(
        r#"cmake_minimum_required(VERSION 3.15)
project({} C)

set(CMAKE_C_STANDARD 11)

add_library({} SHARED
    src/main.c
)

target_include_directories({} PRIVATE
    ${{HSC_PLUGINS_INCLUDE_DIR}}
)

set_target_properties({} PROPERTIES
    PREFIX ""
    OUTPUT_NAME "{}"
)
"#,
        name,
        name.replace('.', "_"),
        name.replace('.', "_"),
        name.replace('.', "_"),
        name.replace('.', "_")
    );
    
    std::fs::write(plugin_dir.join("CMakeLists.txt"), cmake)?;
    
    println!("Created plugin template at: {}", plugin_dir.display());
    println!("Next steps:");
    println!("  1. Edit plugin.toml to configure your plugin");
    println!("  2. Implement the plugin interface in src/main.c");
    println!("  3. Build with: cmake -B build && cmake --build build");
    
    Ok(())
}

fn check_deps(manager: &PluginManager, plugin: &str) -> anyhow::Result<()> {
    let registry = manager.registry();
    
    println!("Dependencies for '{}':", plugin);
    println!("{:-<40}", "");
    
    match registry.get_all_dependencies(plugin) {
        Ok(deps) => {
            if deps.is_empty() {
                println!("No dependencies.");
            } else {
                for dep in &deps {
                    let status = if manager.is_loaded(dep) {
                        "[LOADED]"
                    } else if registry.contains(dep) {
                        "[REGISTERED]"
                    } else {
                        "[MISSING]"
                    };
                    println!("  {} {}", status, dep);
                }
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
    
    Ok(())
}
