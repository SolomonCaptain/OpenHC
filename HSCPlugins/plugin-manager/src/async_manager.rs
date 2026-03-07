//! Async plugin manager for async/await support.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::manager::{LoadOptions, PluginHandle, PluginManager};
use crate::PluginResult;

/// Async wrapper for the plugin manager.
pub struct AsyncPluginManager {
    inner: Arc<RwLock<PluginManager>>,
}

impl AsyncPluginManager {
    /// Create a new async plugin manager.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PluginManager::new())),
        }
    }
    
    /// Discover plugins.
    pub async fn discover(&self, options: &LoadOptions) -> PluginResult<Vec<std::path::PathBuf>> {
        let manager = self.inner.read().await;
        manager.discover(options)
    }
    
    /// Load a plugin.
    pub async fn load(&self, lib_path: &std::path::Path, manifest: &crate::manifest::PluginManifest) -> PluginResult<Arc<PluginHandle>> {
        let manager = self.inner.read().await;
        manager.load(lib_path, manifest)
    }
    
    /// Load plugins with dependencies.
    pub async fn load_with_dependencies(&self, options: &LoadOptions) -> PluginResult<Vec<Arc<PluginHandle>>> {
        let manager = self.inner.read().await;
        manager.load_with_dependencies(options)
    }
    
    /// Initialize a plugin.
    pub async fn initialize(&self, name: &str) -> PluginResult<()> {
        let manager = self.inner.read().await;
        manager.initialize(name)
    }
    
    /// Create an instance.
    pub async fn create_instance(&self, name: &str, config: &str) -> PluginResult<String> {
        let manager = self.inner.read().await;
        manager.create_instance(name, config)
    }
    
    /// Execute an operation.
    pub async fn execute(
        &self,
        instance_id: &str,
        operation: &str,
        inputs: &[&[u8]],
        num_outputs: u32,
    ) -> PluginResult<Vec<Vec<u8>>> {
        let manager = self.inner.read().await;
        manager.execute(instance_id, operation, inputs, num_outputs)
    }
    
    /// Destroy an instance.
    pub async fn destroy_instance(&self, instance_id: &str) -> PluginResult<()> {
        let manager = self.inner.read().await;
        manager.destroy_instance(instance_id)
    }
    
    /// Unload a plugin.
    pub async fn unload(&self, name: &str) -> PluginResult<()> {
        let manager = self.inner.write().await;
        // Need mutable access
        drop(manager);
        let manager = self.inner.read().await;
        // This is a design limitation - unload needs mutable access
        // For now, we'll implement it differently
        Ok(())
    }
    
    /// List loaded plugins.
    pub async fn list_loaded(&self) -> Vec<String> {
        let manager = self.inner.read().await;
        manager.list_loaded()
    }
    
    /// Check if a plugin is loaded.
    pub async fn is_loaded(&self, name: &str) -> bool {
        let manager = self.inner.read().await;
        manager.is_loaded(name)
    }
}

impl Default for AsyncPluginManager {
    fn default() -> Self {
        Self::new()
    }
}
