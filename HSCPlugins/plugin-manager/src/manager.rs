//! Plugin manager - core loading and lifecycle management.

use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use libloading::{Library, Symbol};

use crate::context::{HostServices, HostServicesBuilder, PluginContext};
use crate::error::{ErrorCode, PluginError, PluginResult};
use crate::manifest::{PluginManifest, PluginDependency};
use crate::registry::{PluginInfo, PluginRegistry};
use crate::types::PluginCategory;

/// FFI types matching the C ABI.
mod ffi {
    use std::ffi::{c_void, c_int};
    
    /// Plugin info from C ABI.
    #[repr(C)]
    pub struct PluginInfo {
        pub name: *const i8,
        pub version: *const i8,
        pub description: *const i8,
        pub author: *const i8,
        pub license: *const i8,
        pub homepage: *const i8,
        pub category: i32,
        pub capabilities: u32,
        pub depends_on: *const *const i8,
        pub conflicts: *const *const i8,
        pub api_version: u32,
    }
    
    /// Plugin entry point from C ABI.
    #[repr(C)]
    #[derive(Clone)]
    pub struct PluginEntry {
        pub get_info: extern "C" fn() -> *const PluginInfo,
        pub initialize: extern "C" fn(*mut c_void, *const HostServices) -> i32,
        pub create_instance: extern "C" fn(*const i8, *mut *mut c_void) -> i32,
        pub execute: extern "C" fn(*mut c_void, *const i8, *const *mut c_void, u32, *mut *mut c_void, u32) -> i32,
        pub destroy_instance: extern "C" fn(*mut c_void),
        pub configure: extern "C" fn(*mut c_void, *const i8, *const i8) -> i32,
        pub query: extern "C" fn(*mut c_void, *const i8, *mut i8, usize) -> i32,
        pub shutdown: extern "C" fn() -> i32,
        pub struct_size: u32,
    }
    
    /// Host services for C ABI.
    #[repr(C)]
    pub struct HostServices {
        pub log: extern "C" fn(*mut c_void, i32, *const i8),
        pub alloc: extern "C" fn(*mut c_void, usize, usize) -> *mut c_void,
        pub dealloc: extern "C" fn(*mut c_void, *mut c_void),
        pub get_plugin: extern "C" fn(*mut c_void, *const i8) -> *mut c_void,
        pub get_type: extern "C" fn(*mut c_void, *const i8) -> *mut c_void,
        pub create_value: extern "C" fn(*mut c_void, *const i8, *const c_void, usize) -> *mut c_void,
        pub reserved: [*mut c_void; 8],
    }
}

/// Handle to a loaded plugin.
pub struct PluginHandle {
    /// Plugin info
    pub info: PluginInfo,
    /// Loaded library
    library: Library,
    /// Entry point
    entry: ffi::PluginEntry,
    /// Plugin context
    context: Option<Arc<PluginContext>>,
    /// Active instances
    instances: RwLock<HashMap<String, *mut c_void>>,
}

unsafe impl Send for PluginHandle {}
unsafe impl Sync for PluginHandle {}

impl PluginHandle {
    /// Get the plugin name.
    pub fn name(&self) -> &str {
        &self.info.name
    }
    
    /// Check if the plugin is initialized.
    pub fn is_initialized(&self) -> bool {
        self.context.is_some()
    }
    
    /// Get the number of active instances.
    pub fn instance_count(&self) -> usize {
        self.instances.read().unwrap().len()
    }
}

impl Drop for PluginHandle {
    fn drop(&mut self) {
        // Destroy all instances
        let instance_ids: Vec<String> = self.instances.read().unwrap().keys().cloned().collect();
        for id in instance_ids {
            if let Some(instance) = self.instances.write().unwrap().remove(&id) {
                if !instance.is_null() {
                    (self.entry.destroy_instance)(instance);
                }
            }
        }
        
        // Shutdown plugin
        let _ = (self.entry.shutdown)();
    }
}

/// Options for loading plugins.
#[derive(Debug, Clone)]
pub struct LoadOptions {
    /// Search directories for plugins
    pub search_paths: Vec<PathBuf>,
    /// Only load plugins matching these categories
    pub categories: Option<Vec<PluginCategory>>,
    /// Skip dependency resolution
    pub skip_dependencies: bool,
    /// Allow loading plugins with version mismatches
    pub allow_version_mismatch: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            search_paths: vec![PathBuf::from("./plugins")],
            categories: None,
            skip_dependencies: false,
            allow_version_mismatch: false,
        }
    }
}

/// The plugin manager.
pub struct PluginManager {
    /// Plugin registry
    registry: Arc<PluginRegistry>,
    /// Loaded plugins
    loaded: RwLock<HashMap<String, Arc<PluginHandle>>>,
    /// Host services
    host_services: HostServices,
    /// Load order (for dependency resolution)
    load_order: RwLock<Vec<String>>,
}

impl PluginManager {
    /// Create a new plugin manager.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loaded: RwLock::new(HashMap::new()),
            host_services: HostServicesBuilder::new().build(),
            load_order: RwLock::new(Vec::new()),
        }
    }
    
    /// Create with custom host services.
    pub fn with_services(services: HostServices) -> Self {
        Self {
            registry: Arc::new(PluginRegistry::new()),
            loaded: RwLock::new(HashMap::new()),
            host_services: services,
            load_order: RwLock::new(Vec::new()),
        }
    }
    
    /// Get the plugin registry.
    pub fn registry(&self) -> &Arc<PluginRegistry> {
        &self.registry
    }
    
    /// Discover plugins in the given paths.
    pub fn discover(&self, options: &LoadOptions) -> PluginResult<Vec<PathBuf>> {
        let mut discovered = Vec::new();
        
        for search_path in &options.search_paths {
            if !search_path.exists() {
                continue;
            }
            
            // Look for plugin directories (containing plugin.toml)
            for entry in std::fs::read_dir(search_path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    let manifest_path = path.join(crate::MANIFEST_FILE);
                    if manifest_path.exists() {
                        discovered.push(path);
                    }
                }
            }
        }
        
        Ok(discovered)
    }
    
    /// Load a plugin from a directory.
    pub fn load_from_dir(&self, plugin_dir: &Path) -> PluginResult<Arc<PluginHandle>> {
        let manifest_path = plugin_dir.join(crate::MANIFEST_FILE);
        let manifest = PluginManifest::from_file(&manifest_path)?;
        
        let lib_name = manifest.library_name();
        let lib_path = plugin_dir.join(&lib_name);
        
        self.load(&lib_path, &manifest)
    }
    
    /// Load a plugin from a library file.
    pub fn load(&self, lib_path: &Path, manifest: &PluginManifest) -> PluginResult<Arc<PluginHandle>> {
        let plugin_name = manifest.plugin.name.clone();
        
        // Check if already loaded
        {
            let loaded = self.loaded.read().unwrap();
            if loaded.contains_key(&plugin_name) {
                return Err(PluginError::AlreadyLoaded(plugin_name));
            }
        }
        
        // Validate API version
        if let Some(min_api) = manifest.abi.min_api_version {
            if min_api > crate::API_VERSION {
                return Err(PluginError::ApiVersionMismatch {
                    plugin: plugin_name,
                    plugin_api: min_api,
                    host_api: crate::API_VERSION,
                });
            }
        }
        
        // Load library
        let library = unsafe { Library::new(lib_path) }
            .map_err(|e| PluginError::LoadFailed {
                name: plugin_name.clone(),
                reason: e.to_string(),
            })?;
        
        // Get entry point
        let entry: Symbol<ffi::PluginEntry> = unsafe { 
            library.get(crate::ENTRY_POINT_SYMBOL.as_bytes())
        }.map_err(|e| PluginError::LoadFailed {
            name: plugin_name.clone(),
            reason: format!("Entry point not found: {}", e),
        })?;
        let entry = unsafe { (*entry).clone() };
        
        // Get plugin info
        let c_info = (entry.get_info)();
        let info = self.parse_plugin_info(c_info, lib_path)?;
        
        // Create handle
        let handle = Arc::new(PluginHandle {
            info: info.clone(),
            library,
            entry,
            context: None,
            instances: RwLock::new(HashMap::new()),
        });
        
        // Register plugin
        self.registry.register(info)?;
        
        // Store handle
        {
            let mut loaded = self.loaded.write().unwrap();
            loaded.insert(plugin_name.clone(), handle.clone());
        }
        
        // Update load order
        {
            let mut order = self.load_order.write().unwrap();
            order.push(plugin_name);
        }
        
        Ok(handle)
    }
    
    /// Initialize a plugin.
    pub fn initialize(&self, name: &str) -> PluginResult<()> {
        let handle = self.get_handle(name)?;
        
        if handle.context.is_some() {
            return Err(PluginError::AlreadyInitialized(name.to_string()));
        }
        
        // Create context
        let context = Arc::new(PluginContext::new(name.to_string(), self.host_services.clone()));
        
        // Call initialize
        let result = unsafe {
            (handle.entry.initialize)(
                std::ptr::null_mut(), // context ptr
                std::ptr::null(),     // host services ptr
            )
        };
        
        if result != 0 {
            return Err(PluginError::InitializationFailed {
                name: name.to_string(),
                reason: format!("Error code: {}", result),
            });
        }
        
        // Store context
        {
            let mut loaded = self.loaded.write().unwrap();
            if let Some(h) = loaded.get_mut(name) {
                // Need to use Arc::get_mut for this
                // For now, we'll mark it as initialized in the registry
            }
        }
        
        self.registry.set_loaded(name, true);
        
        Ok(())
    }
    
    /// Create a plugin instance.
    pub fn create_instance(&self, name: &str, config: &str) -> PluginResult<String> {
        let handle = self.get_handle(name)?;
        
        let config_c = CString::new(config).unwrap();
        let mut instance_ptr: *mut c_void = std::ptr::null_mut();
        
        let result = unsafe {
            (handle.entry.create_instance)(
                config_c.as_ptr(),
                &mut instance_ptr,
            )
        };
        
        if result != 0 {
            return Err(PluginError::ExecutionFailed(format!(
                "Failed to create instance: error code {}", result
            )));
        }
        
        let instance_id = format!("{}_{}", name, uuid::Uuid::new_v4());
        
        {
            let mut instances = handle.instances.write().unwrap();
            instances.insert(instance_id.clone(), instance_ptr);
        }
        
        Ok(instance_id)
    }
    
    /// Execute a plugin operation.
    pub fn execute(
        &self,
        instance_id: &str,
        operation: &str,
        inputs: &[&[u8]],
        num_outputs: u32,
    ) -> PluginResult<Vec<Vec<u8>>> {
        // Find the plugin and instance
        let (name, handle, instance_ptr) = self.find_instance(instance_id)?;
        
        let op_c = CString::new(operation).unwrap();
        
        // Prepare outputs
        let mut outputs: Vec<Vec<u8>> = Vec::with_capacity(num_outputs as usize);
        
        let result = unsafe {
            (handle.entry.execute)(
                instance_ptr,
                op_c.as_ptr(),
                std::ptr::null(), // inputs
                inputs.len() as u32,
                std::ptr::null_mut(), // outputs
                num_outputs,
            )
        };
        
        if result != 0 {
            return Err(PluginError::ExecutionFailed(format!(
                "Operation '{}' failed with error code {}", operation, result
            )));
        }
        
        Ok(outputs)
    }
    
    /// Destroy a plugin instance.
    pub fn destroy_instance(&self, instance_id: &str) -> PluginResult<()> {
        let (name, handle, instance_ptr) = self.find_instance(instance_id)?;
        
        if !instance_ptr.is_null() {
            unsafe {
                (handle.entry.destroy_instance)(instance_ptr);
            }
        }
        
        {
            let mut instances = handle.instances.write().unwrap();
            instances.remove(instance_id);
        }
        
        Ok(())
    }
    
    /// Configure a plugin instance.
    pub fn configure(&self, instance_id: &str, key: &str, value: &str) -> PluginResult<()> {
        let (name, handle, instance_ptr) = self.find_instance(instance_id)?;
        
        let key_c = CString::new(key).unwrap();
        let value_c = CString::new(value).unwrap();
        
        let result = unsafe {
            (handle.entry.configure)(
                instance_ptr,
                key_c.as_ptr(),
                value_c.as_ptr(),
            )
        };
        
        if result != 0 {
            return Err(PluginError::ExecutionFailed(format!(
                "Configuration failed with error code {}", result
            )));
        }
        
        Ok(())
    }
    
    /// Query plugin status.
    pub fn query(&self, instance_id: &str, query_name: &str) -> PluginResult<String> {
        let (name, handle, instance_ptr) = self.find_instance(instance_id)?;
        
        let query_c = CString::new(query_name).unwrap();
        let mut result_buf = vec![0i8; 4096];
        
        let result = unsafe {
            (handle.entry.query)(
                instance_ptr,
                query_c.as_ptr(),
                result_buf.as_mut_ptr(),
                result_buf.len(),
            )
        };
        
        if result != 0 {
            return Err(PluginError::ExecutionFailed(format!(
                "Query failed with error code {}", result
            )));
        }
        
        let result_str = unsafe {
            CStr::from_ptr(result_buf.as_ptr())
                .to_string_lossy()
                .into_owned()
        };
        
        Ok(result_str)
    }
    
    /// Unload a plugin.
    pub fn unload(&self, name: &str) -> PluginResult<()> {
        // Check for dependent plugins
        let loaded = self.loaded.read().unwrap();
        for (loaded_name, handle) in loaded.iter() {
            if handle.info.dependencies.contains(&name.to_string()) {
                return Err(PluginError::UnloadFailed {
                    name: name.to_string(),
                    reason: format!("Plugin '{}' depends on it", loaded_name),
                });
            }
        }
        drop(loaded);
        
        // Remove from loaded
        let handle = {
            let mut loaded = self.loaded.write().unwrap();
            loaded.remove(name)
        };
        
        if let Some(handle) = handle {
            // Shutdown will be called in Drop
            self.registry.set_loaded(name, false);
            self.registry.unregister(name)?;
        }
        
        Ok(())
    }
    
    /// Load plugins with dependency resolution.
    pub fn load_with_dependencies(&self, options: &LoadOptions) -> PluginResult<Vec<Arc<PluginHandle>>> {
        let discovered = self.discover(options)?;
        let mut manifests = HashMap::new();
        let mut paths = HashMap::new();
        
        // Load all manifests
        for path in &discovered {
            let manifest_path = path.join(crate::MANIFEST_FILE);
            if let Ok(manifest) = PluginManifest::from_file(&manifest_path) {
                let name = manifest.plugin.name.clone();
                manifests.insert(name.clone(), manifest);
                paths.insert(name, path.clone());
            }
        }
        
        // Topological sort for dependency order
        let order = self.topological_sort(&manifests)?;
        
        // Load in order
        let mut loaded = Vec::new();
        for name in order {
            if let (Some(manifest), Some(path)) = (manifests.get(&name), paths.get(&name)) {
                match self.load(path, manifest) {
                    Ok(handle) => loaded.push(handle),
                    Err(e) => {
                        if !options.allow_version_mismatch {
                            return Err(e);
                        }
                        // Log and continue
                        eprintln!("Warning: Failed to load plugin '{}': {}", name, e);
                    }
                }
            }
        }
        
        Ok(loaded)
    }
    
    /// Get a loaded plugin handle.
    pub fn get_handle(&self, name: &str) -> PluginResult<Arc<PluginHandle>> {
        let loaded = self.loaded.read().unwrap();
        loaded.get(name).cloned()
            .ok_or_else(|| PluginError::NotFound(name.to_string()))
    }
    
    /// List loaded plugins.
    pub fn list_loaded(&self) -> Vec<String> {
        let loaded = self.loaded.read().unwrap();
        loaded.keys().cloned().collect()
    }
    
    /// Check if a plugin is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        let loaded = self.loaded.read().unwrap();
        loaded.contains_key(name)
    }
    
    // Helper methods
    
    fn parse_plugin_info(&self, c_info: *const ffi::PluginInfo, lib_path: &Path) -> PluginResult<PluginInfo> {
        if c_info.is_null() {
            return Err(PluginError::LoadFailed {
                name: "unknown".to_string(),
                reason: "Null plugin info".to_string(),
            });
        }
        
        let info = unsafe { &*c_info };
        
        let name = unsafe { CStr::from_ptr(info.name) }
            .to_string_lossy()
            .into_owned();
        
        let version_str = unsafe { CStr::from_ptr(info.version) }
            .to_string_lossy()
            .into_owned();
        let version = crate::types::Version::parse(&version_str)
            .ok_or_else(|| PluginError::InvalidManifest(format!("Invalid version: {}", version_str)))?;
        
        Ok(PluginInfo {
            name: name.clone(),
            version,
            description: unsafe { CStr::from_ptr(info.description) }
                .to_string_lossy()
                .into_owned(),
            author: unsafe { CStr::from_ptr(info.author) }
                .to_string_lossy()
                .into_owned(),
            license: unsafe { CStr::from_ptr(info.license) }
                .to_string_lossy()
                .into_owned(),
            category: PluginCategory::from(info.category),
            capabilities: crate::types::PluginCapability::from_bits_truncate(info.capabilities),
            resources: crate::types::ResourceRequirements::default(),
            operations: vec![],
            types: vec![],
            dependencies: self.parse_string_array(info.depends_on),
            library_path: lib_path.to_string_lossy().into_owned(),
            is_loaded: false,
        })
    }
    
    fn parse_string_array(&self, ptr: *const *const i8) -> Vec<String> {
        if ptr.is_null() {
            return vec![];
        }
        
        let mut result = Vec::new();
        let mut i = 0;
        
        loop {
            unsafe {
                let str_ptr = *ptr.add(i);
                if str_ptr.is_null() {
                    break;
                }
                result.push(CStr::from_ptr(str_ptr).to_string_lossy().into_owned());
            }
            i += 1;
        }
        
        result
    }
    
    fn find_instance(&self, instance_id: &str) -> PluginResult<(String, Arc<PluginHandle>, *mut c_void)> {
        let loaded = self.loaded.read().unwrap();
        
        for (name, handle) in loaded.iter() {
            let instances = handle.instances.read().unwrap();
            if let Some(&ptr) = instances.get(instance_id) {
                return Ok((name.clone(), handle.clone(), ptr));
            }
        }
        
        Err(PluginError::NotFound(format!("Instance: {}", instance_id)))
    }
    
    fn topological_sort(&self, manifests: &HashMap<String, PluginManifest>) -> PluginResult<Vec<String>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_marks = std::collections::HashSet::new();
        
        for name in manifests.keys() {
            if !visited.contains(name) {
                self.visit(name, manifests, &mut visited, &mut temp_marks, &mut result)?;
            }
        }
        
        Ok(result)
    }
    
    fn visit(
        &self,
        name: &str,
        manifests: &HashMap<String, PluginManifest>,
        visited: &mut std::collections::HashSet<String>,
        temp_marks: &mut std::collections::HashSet<String>,
        result: &mut Vec<String>,
    ) -> PluginResult<()> {
        if temp_marks.contains(name) {
            return Err(PluginError::DependencyCycle(name.to_string()));
        }
        
        if !visited.contains(name) {
            temp_marks.insert(name.to_string());
            
            if let Some(manifest) = manifests.get(name) {
                for dep in &manifest.dependencies {
                    if !dep.optional {
                        self.visit(&dep.name, manifests, visited, temp_marks, result)?;
                    }
                }
            }
            
            temp_marks.remove(name);
            visited.insert(name.to_string());
            result.push(name.to_string());
        }
        
        Ok(())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// Add uuid dependency
mod uuid {
    pub struct Uuid;
    
    impl Uuid {
        pub fn new_v4() -> String {
            format!("{:016x}{:016x}", 
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
                std::process::id() as u64
            )
        }
    }
}
