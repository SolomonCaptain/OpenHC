//! Plugin registry for tracking loaded plugins.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::types::{PluginCapability, PluginCategory, ResourceRequirements, Version};
use crate::manifest::PluginManifest;

/// Information about a loaded plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub name: String,
    /// Plugin version
    pub version: Version,
    /// Human-readable description
    pub description: String,
    /// Author name or organization
    pub author: String,
    /// License identifier
    pub license: String,
    /// Plugin category
    pub category: PluginCategory,
    /// Capability flags
    pub capabilities: PluginCapability,
    /// Resource requirements
    pub resources: ResourceRequirements,
    /// Operations provided by this plugin
    pub operations: Vec<String>,
    /// Types provided by this plugin
    pub types: Vec<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Path to the plugin library
    pub library_path: String,
    /// Whether the plugin is currently loaded
    pub is_loaded: bool,
}

impl PluginInfo {
    /// Create from a manifest.
    pub fn from_manifest(manifest: &PluginManifest, library_path: &str) -> crate::PluginResult<Self> {
        let version = Version::parse(&manifest.plugin.version)
            .ok_or_else(|| crate::PluginError::InvalidManifest(
                format!("Invalid version: {}", manifest.plugin.version)
            ))?;
        
        let category = PluginCategory::from(manifest.plugin.category.as_str());
        let capabilities = PluginCapability::from_strings(&manifest.plugin.capabilities);
        
        Ok(Self {
            name: manifest.plugin.name.clone(),
            version,
            description: manifest.plugin.description.clone(),
            author: manifest.plugin.author.clone(),
            license: manifest.plugin.license.clone(),
            category,
            capabilities,
            resources: ResourceRequirements {
                gpu_memory_mb: manifest.resources.gpu_memory_mb,
                system_memory_mb: manifest.resources.system_memory_mb,
                max_threads: manifest.resources.max_threads,
                compute_units: manifest.resources.compute_units,
            },
            operations: manifest.extensions.operations.clone(),
            types: manifest.extensions.types.clone(),
            dependencies: manifest.dependencies.iter()
                .map(|d| d.name.clone())
                .collect(),
            library_path: library_path.to_string(),
            is_loaded: false,
        })
    }
}

/// Plugin registry for tracking all known plugins.
pub struct PluginRegistry {
    /// Known plugins by name
    plugins: RwLock<HashMap<String, PluginInfo>>,
    /// Operation to plugin mapping
    operation_index: RwLock<HashMap<String, String>>,
    /// Type to plugin mapping
    type_index: RwLock<HashMap<String, String>>,
}

impl PluginRegistry {
    /// Create a new registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            operation_index: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
        }
    }
    
    /// Register a plugin.
    pub fn register(&self, info: PluginInfo) -> crate::PluginResult<()> {
        let name = info.name.clone();
        
        // Update operation index
        {
            let mut ops = self.operation_index.write().unwrap();
            for op in &info.operations {
                ops.insert(op.clone(), name.clone());
            }
        }
        
        // Update type index
        {
            let mut types = self.type_index.write().unwrap();
            for t in &info.types {
                types.insert(t.clone(), name.clone());
            }
        }
        
        // Register plugin
        {
            let mut plugins = self.plugins.write().unwrap();
            plugins.insert(name, info);
        }
        
        Ok(())
    }
    
    /// Unregister a plugin.
    pub fn unregister(&self, name: &str) -> crate::PluginResult<()> {
        let mut plugins = self.plugins.write().unwrap();
        
        if let Some(info) = plugins.remove(name) {
            // Remove from operation index
            let mut ops = self.operation_index.write().unwrap();
            for op in &info.operations {
                ops.remove(op);
            }
            
            // Remove from type index
            let mut types = self.type_index.write().unwrap();
            for t in &info.types {
                types.remove(t);
            }
        }
        
        Ok(())
    }
    
    /// Get plugin info by name.
    pub fn get(&self, name: &str) -> Option<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        plugins.get(name).cloned()
    }
    
    /// Check if a plugin is registered.
    pub fn contains(&self, name: &str) -> bool {
        let plugins = self.plugins.read().unwrap();
        plugins.contains_key(name)
    }
    
    /// Get plugin that provides an operation.
    pub fn get_by_operation(&self, operation: &str) -> Option<PluginInfo> {
        let ops = self.operation_index.read().unwrap();
        ops.get(operation).and_then(|name| self.get(name))
    }
    
    /// Get plugin that provides a type.
    pub fn get_by_type(&self, type_name: &str) -> Option<PluginInfo> {
        let types = self.type_index.read().unwrap();
        types.get(type_name).and_then(|name| self.get(name))
    }
    
    /// List all plugins.
    pub fn list_all(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        plugins.values().cloned().collect()
    }
    
    /// List plugins by category.
    pub fn list_by_category(&self, category: PluginCategory) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        plugins.values()
            .filter(|p| p.category == category)
            .cloned()
            .collect()
    }
    
    /// List loaded plugins.
    pub fn list_loaded(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().unwrap();
        plugins.values()
            .filter(|p| p.is_loaded)
            .cloned()
            .collect()
    }
    
    /// Mark a plugin as loaded.
    pub fn set_loaded(&self, name: &str, loaded: bool) {
        let mut plugins = self.plugins.write().unwrap();
        if let Some(info) = plugins.get_mut(name) {
            info.is_loaded = loaded;
        }
    }
    
    /// Get all dependencies for a plugin (transitive).
    pub fn get_all_dependencies(&self, name: &str) -> crate::PluginResult<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_dependencies(name, &mut result, &mut visited)?;
        Ok(result)
    }
    
    fn collect_dependencies(
        &self,
        name: &str,
        result: &mut Vec<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> crate::PluginResult<()> {
        if visited.contains(name) {
            return Err(crate::PluginError::DependencyCycle(name.to_string()));
        }
        
        visited.insert(name.to_string());
        
        if let Some(info) = self.get(name) {
            for dep in &info.dependencies {
                if !result.contains(dep) {
                    self.collect_dependencies(dep, result, visited)?;
                    result.push(dep.clone());
                }
            }
        }
        
        Ok(())
    }
    
    /// Get the number of registered plugins.
    pub fn len(&self) -> usize {
        let plugins = self.plugins.read().unwrap();
        plugins.len()
    }
    
    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_info(name: &str) -> PluginInfo {
        PluginInfo {
            name: name.to_string(),
            version: Version::new(0, 1, 0),
            description: "Test plugin".to_string(),
            author: "Test".to_string(),
            license: "MIT".to_string(),
            category: PluginCategory::Solver,
            capabilities: PluginCapability::GPU_ACCELERATION,
            resources: ResourceRequirements::default(),
            operations: vec![format!("{}.op", name)],
            types: vec![format!("{}.type", name)],
            dependencies: vec![],
            library_path: format!("/path/to/{}.so", name),
            is_loaded: false,
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = PluginRegistry::new();
        let info = create_test_info("test.plugin");
        
        registry.register(info.clone()).unwrap();
        
        assert!(registry.contains("test.plugin"));
        let retrieved = registry.get("test.plugin").unwrap();
        assert_eq!(retrieved.name, "test.plugin");
    }

    #[test]
    fn test_operation_index() {
        let registry = PluginRegistry::new();
        let mut info = create_test_info("test.plugin");
        info.operations = vec!["custom.operation".to_string()];
        
        registry.register(info).unwrap();
        
        let found = registry.get_by_operation("custom.operation").unwrap();
        assert_eq!(found.name, "test.plugin");
    }

    #[test]
    fn test_dependencies() {
        let registry = PluginRegistry::new();
        
        let mut dep_info = create_test_info("dep.plugin");
        dep_info.dependencies = vec![];
        registry.register(dep_info).unwrap();
        
        let mut main_info = create_test_info("main.plugin");
        main_info.dependencies = vec!["dep.plugin".to_string()];
        registry.register(main_info).unwrap();
        
        let deps = registry.get_all_dependencies("main.plugin").unwrap();
        assert_eq!(deps, vec!["dep.plugin"]);
    }
}
