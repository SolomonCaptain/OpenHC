//! Plugin manifest parsing and validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Plugin manifest structure (parsed from plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginMetadata,
    
    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,
    
    /// ABI configuration
    #[serde(default)]
    pub abi: AbiConfig,
    
    /// Resource requirements
    #[serde(default)]
    pub resources: ResourceRequirements,
    
    /// Extension points
    #[serde(default)]
    pub extensions: Extensions,
}

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier (e.g., "photonics.fdtd")
    pub name: String,
    
    /// Semantic version string
    pub version: String,
    
    /// Human-readable description
    #[serde(default)]
    pub description: String,
    
    /// Author name or organization
    #[serde(default)]
    pub author: String,
    
    /// License identifier (SPDX format)
    #[serde(default = "default_license")]
    pub license: String,
    
    /// Project homepage URL
    #[serde(default)]
    pub homepage: String,
    
    /// Plugin category
    #[serde(default)]
    pub category: String,
    
    /// Capability flags
    #[serde(default)]
    pub capabilities: Vec<String>,
}

fn default_license() -> String {
    "Apache-2.0".to_string()
}

/// Plugin dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency plugin name
    pub name: String,
    
    /// Version requirement (semver range)
    #[serde(default)]
    pub version: Option<String>,
    
    /// Whether this dependency is optional
    #[serde(default)]
    pub optional: bool,
}

/// ABI configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AbiConfig {
    /// Init function symbol name
    #[serde(default = "default_init_func")]
    pub init_func: String,
    
    /// Execute function symbol name
    #[serde(default = "default_execute_func")]
    pub execute_func: String,
    
    /// Cleanup function symbol name
    #[serde(default = "default_cleanup_func")]
    pub cleanup_func: String,
    
    /// Minimum API version required
    #[serde(default)]
    pub min_api_version: Option<u32>,
}

fn default_init_func() -> String {
    "plugin_init".to_string()
}

fn default_execute_func() -> String {
    "plugin_execute".to_string()
}

fn default_cleanup_func() -> String {
    "plugin_cleanup".to_string()
}

/// Resource requirements.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    /// Required GPU memory in MB
    #[serde(default)]
    pub gpu_memory_mb: u64,
    
    /// Required system memory in MB
    #[serde(default)]
    pub system_memory_mb: u64,
    
    /// Maximum threads the plugin can use
    #[serde(default)]
    pub max_threads: u32,
    
    /// Required compute units (GPU/NPU)
    #[serde(default)]
    pub compute_units: u32,
    
    /// Additional requirements as key-value pairs
    #[serde(default)]
    pub additional: HashMap<String, String>,
}

/// Extension points provided by the plugin.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Extensions {
    /// Operations this plugin provides
    #[serde(default)]
    pub operations: Vec<String>,
    
    /// Types this plugin provides
    #[serde(default)]
    pub types: Vec<String>,
    
    /// Configuration schema (JSON Schema)
    #[serde(default)]
    pub config_schema: Option<String>,
}

impl PluginManifest {
    /// Load manifest from a TOML file.
    pub fn from_file(path: &Path) -> crate::PluginResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: PluginManifest = toml::from_str(&content)?;
        manifest.validate()?;
        Ok(manifest)
    }
    
    /// Load manifest from a TOML string.
    pub fn from_toml(toml: &str) -> crate::PluginResult<Self> {
        let manifest: PluginManifest = toml::from_str(toml)?;
        manifest.validate()?;
        Ok(manifest)
    }
    
    /// Validate the manifest.
    pub fn validate(&self) -> crate::PluginResult<()> {
        // Validate name format
        if !self.is_valid_name(&self.plugin.name) {
            return Err(crate::PluginError::InvalidManifest(format!(
                "Invalid plugin name '{}': must be lowercase alphanumeric with dots or underscores",
                self.plugin.name
            )));
        }
        
        // Validate version format
        if !self.is_valid_version(&self.plugin.version) {
            return Err(crate::PluginError::InvalidManifest(format!(
                "Invalid version '{}': must be semver format (e.g., '0.1.0')",
                self.plugin.version
            )));
        }
        
        // Validate category
        let valid_categories = [
            "domain", "solver", "material", "visualization", 
            "backend", "utility", "custom"
        ];
        if !self.plugin.category.is_empty() 
            && !valid_categories.contains(&self.plugin.category.as_str()) {
            return Err(crate::PluginError::InvalidManifest(format!(
                "Invalid category '{}': must be one of {:?}",
                self.plugin.category, valid_categories
            )));
        }
        
        Ok(())
    }
    
    fn is_valid_name(&self, name: &str) -> bool {
        !name.is_empty() 
            && name.len() <= 64
            && name.chars().all(|c| c.is_ascii_lowercase() 
                || c.is_ascii_digit() 
                || c == '.' 
                || c == '_')
    }
    
    fn is_valid_version(&self, version: &str) -> bool {
        // Simple semver validation
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }
        parts.iter().all(|p| p.parse::<u32>().is_ok())
    }
    
    /// Get the plugin library filename.
    pub fn library_name(&self) -> String {
        #[cfg(target_os = "windows")]
        return format!("{}.dll", self.plugin.name.replace('.', "_"));
        
        #[cfg(target_os = "linux")]
        return format!("lib{}.so", self.plugin.name.replace('.', "_"));
        
        #[cfg(target_os = "macos")]
        return format!("lib{}.dylib", self.plugin.name.replace('.', "_"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let toml = r#"
[plugin]
name = "photonics.fdtd"
version = "0.1.0"
description = "FDTD solver for photonics simulation"
author = "OpenHC Team"
category = "solver"

[[dependencies]]
name = "core"
version = ">=0.5.0"

[resources]
gpu_memory_mb = 1024
max_threads = 8

[extensions]
operations = ["photonics.fdtd.update_e", "photonics.fdtd.update_h"]
"#;
        let manifest = PluginManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.plugin.name, "photonics.fdtd");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(manifest.resources.gpu_memory_mb, 1024);
    }

    #[test]
    fn test_invalid_name() {
        let toml = r#"
[plugin]
name = "Invalid-Name"
version = "0.1.0"
"#;
        assert!(PluginManifest::from_toml(toml).is_err());
    }

    #[test]
    fn test_invalid_version() {
        let toml = r#"
[plugin]
name = "test.plugin"
version = "invalid"
"#;
        assert!(PluginManifest::from_toml(toml).is_err());
    }
}
