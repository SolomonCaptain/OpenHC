//! Plugin context and host services.

use std::ffi::{c_void, CStr, CString};
use std::sync::Arc;

/// Plugin context providing access to host services.
pub struct PluginContext {
    /// Plugin name
    name: String,
    /// Host services
    services: HostServices,
    /// User data
    user_data: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl PluginContext {
    /// Create a new plugin context.
    pub fn new(name: String, services: HostServices) -> Self {
        Self {
            name,
            services,
            user_data: None,
        }
    }
    
    /// Get the plugin name.
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get host services.
    pub fn services(&self) -> &HostServices {
        &self.services
    }
    
    /// Set user data.
    pub fn set_user_data<T: std::any::Any + Send + Sync + 'static>(&mut self, data: T) {
        self.user_data = Some(Arc::new(data));
    }
    
    /// Get user data.
    pub fn get_user_data<T: std::any::Any + Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.user_data.as_ref()?.downcast_ref::<T>().cloned()
    }
}

unsafe impl Send for PluginContext {}
unsafe impl Sync for PluginContext {}

/// Host services available to plugins.
pub struct HostServices {
    /// Logging function
    pub log: Option<LogFunc>,
    /// Memory allocation function
    pub alloc: Option<AllocFunc>,
    /// Memory deallocation function
    pub dealloc: Option<DeallocFunc>,
    /// Get plugin function
    pub get_plugin: Option<GetPluginFunc>,
    /// Get type function
    pub get_type: Option<GetTypeFunc>,
    /// Create value function
    pub create_value: Option<CreateValueFunc>,
    /// Custom services
    pub custom: std::collections::HashMap<String, Box<dyn std::any::Any + Send + Sync>>,
}

impl Clone for HostServices {
    fn clone(&self) -> Self {
        Self {
            log: self.log,
            alloc: self.alloc,
            dealloc: self.dealloc,
            get_plugin: self.get_plugin,
            get_type: self.get_type,
            create_value: self.create_value,
            custom: std::collections::HashMap::new(),
        }
    }
}

impl Default for HostServices {
    fn default() -> Self {
        Self {
            log: None,
            alloc: None,
            dealloc: None,
            get_plugin: None,
            get_type: None,
            create_value: None,
            custom: std::collections::HashMap::new(),
        }
    }
}

/// Log function type.
pub type LogFunc = fn(level: LogLevel, message: &str);

/// Allocation function type.
pub type AllocFunc = fn(size: usize, alignment: usize) -> *mut u8;

/// Deallocation function type.
pub type DeallocFunc = fn(ptr: *mut u8);

/// Get plugin function type.
pub type GetPluginFunc = fn(name: &str) -> Option<*mut c_void>;

/// Get type function type.
pub type GetTypeFunc = fn(name: &str) -> Option<*mut c_void>;

/// Create value function type.
pub type CreateValueFunc = fn(type_name: &str, data: &[u8]) -> Option<*mut c_void>;

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl From<i32> for LogLevel {
    fn from(level: i32) -> Self {
        match level {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Default host services implementation.
pub struct DefaultHostServices;

impl DefaultHostServices {
    /// Create default host services.
    pub fn create() -> HostServices {
        HostServices {
            log: Some(Self::default_log),
            alloc: Some(Self::default_alloc),
            dealloc: Some(Self::default_dealloc),
            get_plugin: None,
            get_type: None,
            create_value: None,
            custom: std::collections::HashMap::new(),
        }
    }
    
    fn default_log(level: LogLevel, message: &str) {
        eprintln!("[{}] {}", level, message);
    }
    
    fn default_alloc(size: usize, _alignment: usize) -> *mut u8 {
        let mut v = Vec::with_capacity(size);
        let ptr = v.as_mut_ptr();
        std::mem::forget(v);
        ptr
    }
    
    fn default_dealloc(ptr: *mut u8) {
        if !ptr.is_null() {
            unsafe {
                let _ = Vec::from_raw_parts(ptr, 0, 0);
            }
        }
    }
}

/// Builder for host services.
pub struct HostServicesBuilder {
    services: HostServices,
}

impl HostServicesBuilder {
    pub fn new() -> Self {
        Self {
            services: DefaultHostServices::create(),
        }
    }
    
    pub fn with_log(mut self, func: LogFunc) -> Self {
        self.services.log = Some(func);
        self
    }
    
    pub fn with_alloc(mut self, func: AllocFunc) -> Self {
        self.services.alloc = Some(func);
        self
    }
    
    pub fn with_dealloc(mut self, func: DeallocFunc) -> Self {
        self.services.dealloc = Some(func);
        self
    }
    
    pub fn with_get_plugin(mut self, func: GetPluginFunc) -> Self {
        self.services.get_plugin = Some(func);
        self
    }
    
    pub fn with_get_type(mut self, func: GetTypeFunc) -> Self {
        self.services.get_type = Some(func);
        self
    }
    
    pub fn with_create_value(mut self, func: CreateValueFunc) -> Self {
        self.services.create_value = Some(func);
        self
    }
    
    pub fn with_custom<T: std::any::Any + Send + Sync + 'static>(
        mut self, 
        name: &str, 
        service: T
    ) -> Self {
        self.services.custom.insert(name.to_string(), Box::new(service));
        self
    }
    
    pub fn build(self) -> HostServices {
        self.services
    }
}

impl Default for HostServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_services_builder() {
        let services = HostServicesBuilder::new()
            .with_log(|level, msg| println!("[{}] {}", level, msg))
            .build();
        
        assert!(services.log.is_some());
        assert!(services.alloc.is_some());
    }
}
