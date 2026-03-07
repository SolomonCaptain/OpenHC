//! Plugin type definitions.

use serde::{Deserialize, Serialize};

/// Plugin category enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginCategory {
    /// Domain simulation (photonics, CFD, FEA, etc.)
    Domain,
    /// Numerical solvers (FDTD, FEM, FVM, etc.)
    Solver,
    /// Material libraries
    Material,
    /// Visualization modules
    Visualization,
    /// Backend accelerators (GPU, NPU, FPGA)
    Backend,
    /// Utility plugins (logging, profiling)
    Utility,
    /// Custom category
    Custom,
}

impl Default for PluginCategory {
    fn default() -> Self {
        PluginCategory::Utility
    }
}

impl From<&str> for PluginCategory {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "domain" => PluginCategory::Domain,
            "solver" => PluginCategory::Solver,
            "material" => PluginCategory::Material,
            "visualization" => PluginCategory::Visualization,
            "backend" => PluginCategory::Backend,
            "utility" => PluginCategory::Utility,
            _ => PluginCategory::Custom,
        }
    }
}

impl From<i32> for PluginCategory {
    fn from(category: i32) -> Self {
        match category {
            0 => PluginCategory::Domain,
            1 => PluginCategory::Solver,
            2 => PluginCategory::Material,
            3 => PluginCategory::Visualization,
            4 => PluginCategory::Backend,
            5 => PluginCategory::Utility,
            _ => PluginCategory::Custom,
        }
    }
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginCategory::Domain => write!(f, "domain"),
            PluginCategory::Solver => write!(f, "solver"),
            PluginCategory::Material => write!(f, "material"),
            PluginCategory::Visualization => write!(f, "visualization"),
            PluginCategory::Backend => write!(f, "backend"),
            PluginCategory::Utility => write!(f, "utility"),
            PluginCategory::Custom => write!(f, "custom"),
        }
    }
}

bitflags::bitflags! {
    /// Plugin capability flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PluginCapability: u32 {
        /// No special capabilities
        const NONE = 0;
        /// Supports async execution
        const ASYNC_EXECUTION = 1 << 0;
        /// Supports streaming data
        const STREAMING = 1 << 1;
        /// Can use GPU acceleration
        const GPU_ACCELERATION = 1 << 2;
        /// Can use NPU acceleration
        const NPU_ACCELERATION = 1 << 3;
        /// Can use FPGA acceleration
        const FPGA_ACCELERATION = 1 << 4;
        /// Thread-safe
        const MULTITHREADED = 1 << 5;
        /// Maintains internal state
        const STATEFUL = 1 << 6;
        /// Runtime configurable
        const CONFIGURABLE = 1 << 7;
    }
}

impl Default for PluginCapability {
    fn default() -> Self {
        PluginCapability::NONE
    }
}

impl PluginCapability {
    /// Parse capabilities from a list of strings.
    pub fn from_strings(strs: &[String]) -> Self {
        let mut caps = PluginCapability::NONE;
        for s in strs {
            match s.to_lowercase().as_str() {
                "async_execution" => caps |= PluginCapability::ASYNC_EXECUTION,
                "streaming" => caps |= PluginCapability::STREAMING,
                "gpu_acceleration" => caps |= PluginCapability::GPU_ACCELERATION,
                "npu_acceleration" => caps |= PluginCapability::NPU_ACCELERATION,
                "fpga_acceleration" => caps |= PluginCapability::FPGA_ACCELERATION,
                "multithreaded" => caps |= PluginCapability::MULTITHREADED,
                "stateful" => caps |= PluginCapability::STATEFUL,
                "configurable" => caps |= PluginCapability::CONFIGURABLE,
                _ => {}
            }
        }
        caps
    }
}

/// Resource requirements for a plugin.
#[derive(Debug, Clone, Default)]
pub struct ResourceRequirements {
    /// Required GPU memory in MB
    pub gpu_memory_mb: u64,
    /// Required system memory in MB
    pub system_memory_mb: u64,
    /// Maximum threads the plugin can use
    pub max_threads: u32,
    /// Required compute units (GPU/NPU)
    pub compute_units: u32,
}

/// Plugin version information.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Version {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
    
    pub fn to_u32(&self) -> u32 {
        self.major * 10000 + self.minor * 100 + self.patch
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for Version {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Version::parse(s).ok_or_else(|| format!("Invalid version: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 1, 0);
        let v3 = Version::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_capability_flags() {
        let caps = PluginCapability::from_strings(&[
            "gpu_acceleration".to_string(),
            "multithreaded".to_string(),
        ]);
        assert!(caps.contains(PluginCapability::GPU_ACCELERATION));
        assert!(caps.contains(PluginCapability::MULTITHREADED));
        assert!(!caps.contains(PluginCapability::NPU_ACCELERATION));
    }
}
