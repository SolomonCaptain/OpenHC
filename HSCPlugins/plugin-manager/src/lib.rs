//! OpenHC Plugin System - Plugin Manager
//!
//! This crate provides the plugin management infrastructure for OpenHC.
//! It handles plugin discovery, loading, lifecycle management, and 
//! inter-plugin communication.

pub mod error;
pub mod manifest;
pub mod manager;
pub mod registry;
pub mod context;
pub mod types;

#[cfg(feature = "async")]
pub mod async_manager;

// Re-export main types
pub use error::{PluginError, PluginResult};
pub use manifest::{PluginManifest, PluginDependency};
pub use manager::{PluginManager, PluginHandle, LoadOptions};
pub use registry::{PluginRegistry, PluginInfo};
pub use context::{PluginContext, HostServices};
pub use types::{PluginCategory, PluginCapability};

/// Current API version
pub const API_VERSION: u32 = 10000;

/// Plugin file extension by platform
#[cfg(target_os = "windows")]
pub const PLUGIN_EXTENSION: &str = "dll";

#[cfg(target_os = "linux")]
pub const PLUGIN_EXTENSION: &str = "so";

#[cfg(target_os = "macos")]
pub const PLUGIN_EXTENSION: &str = "dylib";

/// Plugin manifest filename
pub const MANIFEST_FILE: &str = "plugin.toml";

/// Plugin entry point symbol name
pub const ENTRY_POINT_SYMBOL: &str = "HSC_PLUGIN_ENTRY";
