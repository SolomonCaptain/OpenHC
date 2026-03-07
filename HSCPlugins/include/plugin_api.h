/**
 * @file plugin_api.h
 * @brief OpenHC Plugin System - Core API Definition (C ABI)
 * 
 * This header defines the stable C ABI interface for OpenHC plugins.
 * All plugins must implement these interfaces to be loadable by the
 * plugin manager.
 * 
 * @version 1.0.0
 * @author OpenHC Team
 */

#ifndef HSC_PLUGINS_API_H
#define HSC_PLUGINS_API_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Version and Compatibility
 * ============================================================================ */

#define HSC_PLUGIN_API_VERSION_MAJOR 1
#define HSC_PLUGIN_API_VERSION_MINOR 0
#define HSC_PLUGIN_API_VERSION_PATCH 0

/**
 * Plugin API version as a single integer for easy comparison.
 * Format: MAJOR * 10000 + MINOR * 100 + PATCH
 */
#define HSC_PLUGIN_API_VERSION 10000

/* ============================================================================
 * Type Definitions
 * ============================================================================ */

/**
 * Error codes returned by plugin operations.
 */
typedef enum HscErrorCode {
    HSC_SUCCESS = 0,
    HSC_ERROR_UNKNOWN = -1,
    HSC_ERROR_INVALID_ARGUMENT = -2,
    HSC_ERROR_OUT_OF_MEMORY = -3,
    HSC_ERROR_NOT_INITIALIZED = -4,
    HSC_ERROR_ALREADY_INITIALIZED = -5,
    HSC_ERROR_OPERATION_NOT_SUPPORTED = -6,
    HSC_ERROR_DEPENDENCY_MISSING = -7,
    HSC_ERROR_VERSION_MISMATCH = -8,
    HSC_ERROR_RESOURCE_EXHAUSTED = -9,
    HSC_ERROR_TIMEOUT = -10,
    HSC_ERROR_INTERNAL = -11,
} HscErrorCode;

/**
 * Plugin category determines the extension point.
 */
typedef enum HscPluginCategory {
    HSC_CATEGORY_DOMAIN = 0,        // Domain simulation (photonics, CFD, FEA, etc.)
    HSC_CATEGORY_SOLVER = 1,        // Numerical solvers (FDTD, FEM, FVM, etc.)
    HSC_CATEGORY_MATERIAL = 2,      // Material libraries
    HSC_CATEGORY_VISUALIZATION = 3, // Visualization modules
    HSC_CATEGORY_BACKEND = 4,       // Backend accelerators (GPU, NPU, FPGA)
    HSC_CATEGORY_UTILITY = 5,       // Utility plugins (logging, profiling)
    HSC_CATEGORY_CUSTOM = 99,       // Custom category
} HscPluginCategory;

/**
 * Plugin capability flags.
 */
typedef enum HscPluginCapability {
    HSC_CAP_NONE = 0,
    HSC_CAP_ASYNC_EXECUTION = 1 << 0,    // Supports async execution
    HSC_CAP_STREAMING = 1 << 1,          // Supports streaming data
    HSC_CAP_GPU_ACCELERATION = 1 << 2,   // Can use GPU
    HSC_CAP_NPU_ACCELERATION = 1 << 3,   // Can use NPU
    HSC_CAP_FPGA_ACCELERATION = 1 << 4,  // Can use FPGA
    HSC_CAP_MULTITHREADED = 1 << 5,      // Thread-safe
    HSC_CAP_STATEFUL = 1 << 6,           // Maintains internal state
    HSC_CAP_CONFIGURABLE = 1 << 7,       // Runtime configurable
} HscPluginCapability;

/**
 * Resource requirements for a plugin.
 */
typedef struct HscResourceRequirements {
    uint64_t gpu_memory_mb;      // Required GPU memory in MB
    uint64_t system_memory_mb;   // Required system memory in MB
    uint32_t max_threads;        // Maximum threads the plugin can use
    uint32_t compute_units;      // Required compute units (GPU/NPU)
    const char* additional_reqs; // JSON string of additional requirements
} HscResourceRequirements;

/**
 * Plugin metadata structure.
 * This is the primary information exposed by each plugin.
 */
typedef struct HscPluginInfo {
    // Identity
    const char* name;           // Unique plugin identifier (e.g., "photonics.fdtd")
    const char* version;        // Semantic version string (e.g., "0.1.0")
    const char* description;    // Human-readable description
    const char* author;         // Author name or organization
    const char* license;        // License identifier (SPDX format)
    const char* homepage;       // Project homepage URL
    
    // Classification
    HscPluginCategory category;
    uint32_t capabilities;      // Bitwise OR of HscPluginCapability
    
    // Dependencies
    const char* const* depends_on;  // NULL-terminated array of plugin names
    const char* const* conflicts;   // NULL-terminated array of conflicting plugins
    uint32_t api_version;           // API version this plugin was built against
    
    // Resources
    HscResourceRequirements resources;
    
    // Extension points
    const char* const* provides_operations; // Operations this plugin provides
    const char* const* provides_types;      // Types this plugin provides
    
} HscPluginInfo;

/* ============================================================================
 * Context and Handle Types
 * ============================================================================ */

/**
 * Opaque handle to the plugin context provided by the host.
 * The context provides access to host services and shared resources.
 */
typedef struct HscPluginContext HscPluginContext;

/**
 * Opaque handle to a plugin instance.
 * Each plugin may have multiple instances.
 */
typedef struct HscPluginInstance HscPluginInstance;

/**
 * Opaque handle to a value in the HSCIR type system.
 */
typedef struct HscValue HscValue;

/**
 * Opaque handle to an operation in the HSCIR system.
 */
typedef struct HscOperation HscOperation;

/**
 * Opaque handle to a type in the HSCIR type system.
 */
typedef struct HscType HscType;

/* ============================================================================
 * Host-provided Services (via Context)
 * ============================================================================ */

/**
 * Function pointer type for logging.
 */
typedef void (*HscLogFunc)(
    HscPluginContext* ctx,
    int level,              // 0=trace, 1=debug, 2=info, 3=warn, 4=error
    const char* message
);

/**
 * Function pointer type for memory allocation.
 */
typedef void* (*HscAllocFunc)(
    HscPluginContext* ctx,
    size_t size,
    size_t alignment
);

/**
 * Function pointer type for memory deallocation.
 */
typedef void (*HscDeallocFunc)(
    HscPluginContext* ctx,
    void* ptr
);

/**
 * Function pointer type for getting another plugin's interface.
 */
typedef HscPluginInstance* (*HscGetPluginFunc)(
    HscPluginContext* ctx,
    const char* plugin_name
);

/**
 * Function pointer type for type operations.
 */
typedef HscType* (*HscGetTypeFunc)(
    HscPluginContext* ctx,
    const char* type_name
);

/**
 * Function pointer type for creating values.
 */
typedef HscValue* (*HscCreateValueFunc)(
    HscPluginContext* ctx,
    HscType* type,
    const void* data,
    size_t size
);

/**
 * Host services available to plugins through the context.
 */
typedef struct HscHostServices {
    HscLogFunc log;
    HscAllocFunc alloc;
    HscDeallocFunc dealloc;
    HscGetPluginFunc get_plugin;
    HscGetTypeFunc get_type;
    HscCreateValueFunc create_value;
    void* reserved[8];  // Reserved for future expansion
} HscHostServices;

/* ============================================================================
 * Plugin Interface (Must be implemented by each plugin)
 * ============================================================================ */

/**
 * Called immediately after loading to get plugin metadata.
 * This function must be implemented and exported by every plugin.
 * 
 * @return Pointer to a static HscPluginInfo structure.
 *         The returned pointer must remain valid for the plugin's lifetime.
 */
typedef const HscPluginInfo* (*HscPluginGetInfoFunc)(void);

/**
 * Called to initialize the plugin with host context.
 * 
 * @param context The plugin context providing access to host services.
 * @param services Table of host service functions.
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginInitializeFunc)(
    HscPluginContext* context,
    const HscHostServices* services
);

/**
 * Called to create a new plugin instance.
 * Plugins may support multiple concurrent instances.
 * 
 * @param config JSON string with instance configuration.
 * @param instance Output pointer to the created instance.
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginCreateInstanceFunc)(
    const char* config,
    HscPluginInstance** instance
);

/**
 * Called to execute a plugin operation.
 * 
 * @param instance The plugin instance.
 * @param operation_name Name of the operation to execute.
 * @param inputs Array of input values.
 * @param num_inputs Number of input values.
 * @param outputs Array to receive output values.
 * @param num_outputs Number of expected output values.
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginExecuteFunc)(
    HscPluginInstance* instance,
    const char* operation_name,
    const HscValue* const* inputs,
    uint32_t num_inputs,
    HscValue** outputs,
    uint32_t num_outputs
);

/**
 * Called to destroy a plugin instance.
 * 
 * @param instance The instance to destroy.
 */
typedef void (*HscPluginDestroyInstanceFunc)(
    HscPluginInstance* instance
);

/**
 * Called to configure a running instance.
 * 
 * @param instance The plugin instance.
 * @param config_key Configuration key to set.
 * @param config_value Configuration value (JSON string).
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginConfigureFunc)(
    HscPluginInstance* instance,
    const char* config_key,
    const char* config_value
);

/**
 * Called to query plugin status or metrics.
 * 
 * @param instance The plugin instance.
 * @param query_name Name of the query.
 * @param result Output buffer for result (JSON string).
 * @param result_size Size of result buffer.
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginQueryFunc)(
    HscPluginInstance* instance,
    const char* query_name,
    char* result,
    size_t result_size
);

/**
 * Called before unloading the plugin.
 * 
 * @return HSC_SUCCESS on success, error code otherwise.
 */
typedef HscErrorCode (*HscPluginShutdownFunc)(void);

/* ============================================================================
 * Plugin Entry Point Structure
 * ============================================================================ */

/**
 * The plugin entry point table.
 * All plugins must export a symbol named "HSC_PLUGIN_ENTRY" 
 * pointing to this structure.
 */
typedef struct HscPluginEntry {
    HscPluginGetInfoFunc get_info;
    HscPluginInitializeFunc initialize;
    HscPluginCreateInstanceFunc create_instance;
    HscPluginExecuteFunc execute;
    HscPluginDestroyInstanceFunc destroy_instance;
    HscPluginConfigureFunc configure;
    HscPluginQueryFunc query;
    HscPluginShutdownFunc shutdown;
    uint32_t struct_size;  // Size of this structure for versioning
} HscPluginEntry;

/* ============================================================================
 * Helper Macros for Plugin Implementation
 * ============================================================================ */

/**
 * Macro to define and export a plugin entry point.
 * 
 * Usage:
 *   HSC_EXPORT_PLUGIN(
 *       my_get_info,
 *       my_initialize,
 *       my_create_instance,
 *       my_execute,
 *       my_destroy_instance,
 *       my_configure,
 *       my_query,
 *       my_shutdown
 *   )
 */
#ifdef _WIN32
    #define HSC_EXPORT __declspec(dllexport)
#else
    #define HSC_EXPORT __attribute__((visibility("default")))
#endif

#define HSC_EXPORT_PLUGIN(get_info_fn, init_fn, create_fn, exec_fn, destroy_fn, config_fn, query_fn, shutdown_fn) \
    extern "C" HSC_EXPORT const HscPluginEntry HSC_PLUGIN_ENTRY = { \
        .get_info = get_info_fn, \
        .initialize = init_fn, \
        .create_instance = create_fn, \
        .execute = exec_fn, \
        .destroy_instance = destroy_fn, \
        .configure = config_fn, \
        .query = query_fn, \
        .shutdown = shutdown_fn, \
        .struct_size = sizeof(HscPluginEntry), \
    };

/* ============================================================================
 * Photonics-specific Extensions (Optional)
 * ============================================================================ */

/**
 * Photonics-specific operation names.
 * These are provided by photonics domain plugins.
 */
#define HSC_OP_PHOTONICS_FDTD_UPDATE_E   "photonics.fdtd.update_e"
#define HSC_OP_PHOTONICS_FDTD_UPDATE_H   "photonics.fdtd.update_h"
#define HSC_OP_PHOTONICS_APPLY_SOURCE    "photonics.source.apply"
#define HSC_OP_PHOTONICS_APPLY_BOUNDARY  "photonics.boundary.apply"
#define HSC_OP_PHOTONICS_RECORD_MONITOR  "photonics.monitor.record"
#define HSC_OP_PHOTONICS_MATERIAL_INTERP "photonics.material.interpolate"

/**
 * Photonics-specific type names.
 */
#define HSC_TYPE_OPTICAL_FIELD    "photonics.field"
#define HSC_TYPE_MATERIAL         "photonics.material"
#define HSC_TYPE_SPECTRUM         "photonics.spectrum"
#define HSC_TYPE_MODE             "photonics.mode"
#define HSC_TYPE_DOMAIN           "photonics.domain"
#define HSC_TYPE_BOUNDARY         "photonics.boundary"

#ifdef __cplusplus
}
#endif

#endif /* HSC_PLUGINS_API_H */
