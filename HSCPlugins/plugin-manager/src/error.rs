//! Error types for the plugin system.

use thiserror::Error;

/// Result type alias for plugin operations.
pub type PluginResult<T> = Result<T, PluginError>;

/// Error codes matching the C ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    Success = 0,
    Unknown = -1,
    InvalidArgument = -2,
    OutOfMemory = -3,
    NotInitialized = -4,
    AlreadyInitialized = -5,
    OperationNotSupported = -6,
    DependencyMissing = -7,
    VersionMismatch = -8,
    ResourceExhausted = -9,
    Timeout = -10,
    Internal = -11,
}

impl From<i32> for ErrorCode {
    fn from(code: i32) -> Self {
        match code {
            0 => ErrorCode::Success,
            -1 => ErrorCode::Unknown,
            -2 => ErrorCode::InvalidArgument,
            -3 => ErrorCode::OutOfMemory,
            -4 => ErrorCode::NotInitialized,
            -5 => ErrorCode::AlreadyInitialized,
            -6 => ErrorCode::OperationNotSupported,
            -7 => ErrorCode::DependencyMissing,
            -8 => ErrorCode::VersionMismatch,
            -9 => ErrorCode::ResourceExhausted,
            -10 => ErrorCode::Timeout,
            -11 => ErrorCode::Internal,
            _ => ErrorCode::Unknown,
        }
    }
}

impl From<ErrorCode> for i32 {
    fn from(code: ErrorCode) -> Self {
        code as i32
    }
}

/// Plugin system error type.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Failed to load plugin '{name}': {reason}")]
    LoadFailed { name: String, reason: String },

    #[error("Failed to initialize plugin '{name}': {reason}")]
    InitializationFailed { name: String, reason: String },

    #[error("Failed to unload plugin '{name}': {reason}")]
    UnloadFailed { name: String, reason: String },

    #[error("Dependency cycle detected involving: {0}")]
    DependencyCycle(String),

    #[error("Missing dependency '{dependency}' for plugin '{plugin}'")]
    MissingDependency { plugin: String, dependency: String },

    #[error("Version conflict: plugin '{plugin}' requires {required}, but found {found}")]
    VersionConflict {
        plugin: String,
        required: String,
        found: String,
    },

    #[error("API version mismatch: plugin '{plugin}' built with API {plugin_api}, host has {host_api}")]
    ApiVersionMismatch {
        plugin: String,
        plugin_api: u32,
        host_api: u32,
    },

    #[error("Invalid plugin manifest: {0}")]
    InvalidManifest(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Operation not supported: {operation} on plugin {plugin}")]
    OperationNotSupported { plugin: String, operation: String },

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Plugin not initialized: {0}")]
    NotInitialized(String),

    #[error("Plugin already initialized: {0}")]
    AlreadyInitialized(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl PluginError {
    /// Convert to error code.
    pub fn to_error_code(&self) -> ErrorCode {
        match self {
            PluginError::NotFound(_) => ErrorCode::OperationNotSupported,
            PluginError::AlreadyLoaded(_) => ErrorCode::AlreadyInitialized,
            PluginError::LoadFailed { .. } => ErrorCode::Internal,
            PluginError::InitializationFailed { .. } => ErrorCode::NotInitialized,
            PluginError::UnloadFailed { .. } => ErrorCode::Internal,
            PluginError::DependencyCycle(_) => ErrorCode::DependencyMissing,
            PluginError::MissingDependency { .. } => ErrorCode::DependencyMissing,
            PluginError::VersionConflict { .. } => ErrorCode::VersionMismatch,
            PluginError::ApiVersionMismatch { .. } => ErrorCode::VersionMismatch,
            PluginError::InvalidManifest(_) => ErrorCode::InvalidArgument,
            PluginError::ExecutionFailed(_) => ErrorCode::Internal,
            PluginError::OperationNotSupported { .. } => ErrorCode::OperationNotSupported,
            PluginError::ResourceExhausted(_) => ErrorCode::ResourceExhausted,
            PluginError::InvalidArgument(_) => ErrorCode::InvalidArgument,
            PluginError::NotInitialized(_) => ErrorCode::NotInitialized,
            PluginError::AlreadyInitialized(_) => ErrorCode::AlreadyInitialized,
            PluginError::Internal(_) => ErrorCode::Internal,
            PluginError::Io(_) => ErrorCode::Internal,
            PluginError::Serialization(_) => ErrorCode::InvalidArgument,
        }
    }
}

impl From<toml::de::Error> for PluginError {
    fn from(e: toml::de::Error) -> Self {
        PluginError::Serialization(e.to_string())
    }
}

impl From<serde_json::Error> for PluginError {
    fn from(e: serde_json::Error) -> Self {
        PluginError::Serialization(e.to_string())
    }
}
