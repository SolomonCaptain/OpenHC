/**
 * @file photonics_interface.h
 * @brief Photonics Simulation Module - Boundary Interface Definition
 * 
 * This header defines the standardized interfaces for photonics simulation
 * plugins in the OpenHC ecosystem. All photonics-related plugins should
 * implement these interfaces for interoperability.
 * 
 * @version 1.0.0
 * @author OpenHC Team
 */

#ifndef HSC_PHOTONICS_INTERFACE_H
#define HSC_PHOTONICS_INTERFACE_H

#include "plugin_api.h"
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Photonics Type System
 * ============================================================================ */

/**
 * Optical field components.
 */
typedef enum HscFieldComponent {
    HSC_FIELD_EX = 0,    // Electric field X component
    HSC_FIELD_EY = 1,    // Electric field Y component
    HSC_FIELD_EZ = 2,    // Electric field Z component
    HSC_FIELD_HX = 3,    // Magnetic field X component
    HSC_FIELD_HY = 4,    // Magnetic field Y component
    HSC_FIELD_HZ = 5,    // Magnetic field Z component
    HSC_FIELD_ALL = 6,   // All components
} HscFieldComponent;

/**
 * Polarization modes.
 */
typedef enum HscPolarization {
    HSC_POL_TE = 0,      // Transverse electric
    HSC_POL_TM = 1,      // Transverse magnetic
    HSC_POL_TEM = 2,     // Transverse electromagnetic
    HSC_POL_CUSTOM = 3,  // Custom polarization
} HscPolarization;

/**
 * Boundary condition types.
 */
typedef enum HscBoundaryType {
    HSC_BOUNDARY_PML = 0,        // Perfectly matched layer
    HSC_BOUNDARY_PEC = 1,        // Perfect electric conductor
    HSC_BOUNDARY_PMC = 2,        // Perfect magnetic conductor
    HSC_BOUNDARY_PERIODIC = 3,   // Periodic boundary
    HSC_BOUNDARY_ABSORBING = 4,  // Absorbing boundary
    HSC_BOUNDARY_BLOCH = 5,      // Bloch boundary
    HSC_BOUNDARY_SYMMETRIC = 6,  // Symmetric boundary
    HSC_BOUNDARY_ANTI_SYMMETRIC = 7, // Anti-symmetric boundary
} HscBoundaryType;

/**
 * Solver types for photonics simulation.
 */
typedef enum HscSolverType {
    HSC_SOLVER_FDTD = 0,   // Finite-Difference Time-Domain
    HSC_SOLVER_FDE = 1,    // Finite-Difference Eigenmode
    HSC_SOLVER_RCWA = 2,   // Rigorous Coupled-Wave Analysis
    HSC_SOLVER_FEM = 3,    // Finite Element Method
    HSC_SOLVER_BEM = 4,    // Boundary Element Method
    HSC_SOLVER_RAY = 5,    // Ray tracing
    HSC_SOLVER_CUSTOM = 6, // Custom solver
} HscSolverType;

/**
 * Material dispersion models.
 */
typedef enum HscDispersionModel {
    HSC_DISP_NONE = 0,         // No dispersion (constant n)
    HSC_DISP_SELLMEIER = 1,    // Sellmeier equation
    HSC_DISP_POLYNOMIAL = 2,   // Polynomial fit
    HSC_DISP_DRUDE = 3,        // Drude model
    HSC_DISP_LORENTZ = 4,      // Lorentz model
    HSC_DISP_DEBYE = 5,        // Debye model
    HSC_DISP_TABLE = 6,        // Tabulated data
    HSC_DISP_CUSTOM = 7,       // Custom model
} HscDispersionModel;

/**
 * Monitor types for data collection.
 */
typedef enum HscMonitorType {
    HSC_MONITOR_FIELD = 0,     // Field monitor
    HSC_MONITOR_POWER = 1,     // Power monitor
    HSC_MONITOR_SPECTRUM = 2,  // Spectrum monitor
    HSC_MONITOR_MODE = 3,      // Mode monitor
    HSC_MONITOR_MOVIE = 4,     // Movie/animation monitor
    HSC_MONITOR_CUSTOM = 5,    // Custom monitor
} HscMonitorType;

/* ============================================================================
 * Photonics Data Structures
 * ============================================================================ */

/**
 * Complex number representation.
 */
typedef struct HscComplex {
    double real;
    double imag;
} HscComplex;

/**
 * 3D point/vector.
 */
typedef struct HscVec3 {
    double x, y, z;
} HscVec3;

/**
 * 3D integer index.
 */
typedef struct HscIndex3 {
    int64_t i, j, k;
} HscIndex3;

/**
 * 3D bounding box.
 */
typedef struct HscBoundingBox {
    HscVec3 min;
    HscVec3 max;
} HscBoundingBox;

/**
 * Simulation domain definition.
 */
typedef struct HscDomain {
    HscVec3 size;              // Physical size in micrometers
    HscIndex3 mesh;            // Grid resolution
    double time_steps;         // Number of time steps
    double courant_factor;     // Courant stability factor
    HscBoundaryType boundaries[6]; // Boundary conditions for each face
    uint32_t pml_layers;       // PML layer count
    double pml_sigma;          // PML conductivity
} HscDomain;

/**
 * Material definition.
 */
typedef struct HscMaterial {
    const char* name;          // Material name
    double base_refractive_index; // Base refractive index
    HscDispersionModel dispersion;
    double dispersion_params[8]; // Dispersion model parameters
    double loss;               // Material loss (dB/cm)
    bool is_anisotropic;       // Whether material is anisotropic
    HscComplex epsilon[9];     // Permittivity tensor (3x3)
    HscComplex mu[9];          // Permeability tensor (3x3)
} HscMaterial;

/**
 * Source definition.
 */
typedef struct HscSource {
    const char* name;          // Source name
    HscBoundingBox region;     // Source region
    double wavelength;         // Center wavelength (um)
    double bandwidth;          // Bandwidth (um)
    HscPolarization polarization;
    double amplitude;          // Source amplitude
    double phase;              // Initial phase
    const char* time_profile;  // Time profile type (gaussian, cw, pulse)
    double pulse_width;        // Pulse width for transient sources
} HscSource;

/**
 * Monitor definition.
 */
typedef struct HscMonitor {
    const char* name;          // Monitor name
    HscMonitorType type;
    HscBoundingBox region;     // Monitor region
    HscFieldComponent component;
    double frequency_min;      // Min frequency for spectrum
    double frequency_max;      // Max frequency for spectrum
    uint32_t frequency_points; // Number of frequency points
    uint32_t sample_interval;  // Sampling interval
} HscMonitor;

/**
 * Simulation results.
 */
typedef struct HscSimulationResults {
    // Field data (can be NULL if not recorded)
    HscComplex* e_field;       // Electric field (size: mesh.x * mesh.y * mesh.z * 3)
    HscComplex* h_field;       // Magnetic field
    
    // Spectrum data
    double* frequencies;       // Frequency array
    HscComplex* transmission;  // Transmission spectrum
    HscComplex* reflection;    // Reflection spectrum
    
    // Mode data
    uint32_t num_modes;        // Number of modes found
    double* effective_index;   // Effective indices
    HscComplex** mode_fields;  // Mode field profiles
    
    // Metadata
    double simulation_time;    // Wall-clock time
    uint64_t memory_used;      // Memory usage in bytes
    uint32_t time_steps_completed;
    bool converged;
    double final_energy;       // Final field energy
} HscSimulationResults;

/* ============================================================================
 * Photonics Plugin Interface (Extension of base plugin API)
 * ============================================================================ */

/**
 * Photonics domain plugin interface.
 * Extends the base plugin interface with photonics-specific operations.
 */
typedef struct HscPhotonicsInterface {
    // Base interface (must be first)
    HscPluginEntry base;
    
    // Domain creation
    HscErrorCode (*create_domain)(
        HscPluginInstance* instance,
        const HscDomain* domain,
        HscPluginInstance** domain_instance
    );
    
    // Material management
    HscErrorCode (*add_material)(
        HscPluginInstance* instance,
        const HscMaterial* material
    );
    
    HscErrorCode (*set_material_region)(
        HscPluginInstance* instance,
        const char* material_name,
        const HscBoundingBox* region,
        bool additive
    );
    
    // Source management
    HscErrorCode (*add_source)(
        HscPluginInstance* instance,
        const HscSource* source
    );
    
    // Monitor management
    HscErrorCode (*add_monitor)(
        HscPluginInstance* instance,
        const HscMonitor* monitor
    );
    
    // Simulation control
    HscErrorCode (*run_simulation)(
        HscPluginInstance* instance,
        uint64_t max_steps,
        HscSimulationResults** results
    );
    
    HscErrorCode (*step_simulation)(
        HscPluginInstance* instance,
        uint32_t num_steps
    );
    
    HscErrorCode (*reset_simulation)(
        HscPluginInstance* instance
    );
    
    // Results retrieval
    HscErrorCode (*get_field)(
        HscPluginInstance* instance,
        const char* monitor_name,
        HscFieldComponent component,
        HscComplex** field_data,
        HscIndex3* size
    );
    
    HscErrorCode (*get_spectrum)(
        HscPluginInstance* instance,
        const char* monitor_name,
        double** frequencies,
        HscComplex** data,
        uint32_t* num_points
    );
    
    // Utility functions
    HscErrorCode (*refine_mesh)(
        HscPluginInstance* instance,
        const HscBoundingBox* region,
        uint32_t refinement_factor
    );
    
    HscErrorCode (*export_results)(
        HscPluginInstance* instance,
        const char* format,
        const char* path
    );
    
} HscPhotonicsInterface;

/* ============================================================================
 * FDTD-specific Interface
 * ============================================================================ */

/**
 * FDTD solver configuration.
 */
typedef struct HscFDTDConfig {
    double courant_factor;     // Courant factor (default: 0.99)
    double auto_shutoff_min;   // Auto shutoff minimum (default: 1e-5)
    double auto_shutoff_max;   // Auto shutoff maximum (default: 1e5)
    uint32_t pml_layers;       // PML layers (default: 8)
    double pml_sigma_max;      // Maximum PML conductivity
    bool use_dispersion;       // Enable dispersion in FDTD
    bool use_subpixel;         // Enable subpixel smoothing
    double dt;                 // Time step (calculated if 0)
} HscFDTDConfig;

/**
 * FDTD-specific operations.
 */
typedef struct HscFDTDInterface {
    // Base photonics interface
    HscPhotonicsInterface photonics;
    
    // FDTD-specific operations
    HscErrorCode (*update_e_field)(
        HscPluginInstance* instance
    );
    
    HscErrorCode (*update_h_field)(
        HscPluginInstance* instance
    );
    
    HscErrorCode (*apply_sources)(
        HscPluginInstance* instance,
        uint64_t time_step
    );
    
    HscErrorCode (*apply_boundaries)(
        HscPluginInstance* instance
    );
    
    HscErrorCode (*record_monitors)(
        HscPluginInstance* instance,
        uint64_t time_step
    );
    
    double (*calculate_energy)(
        HscPluginInstance* instance
    );
    
    bool (*check_convergence)(
        HscPluginInstance* instance,
        double threshold
    );
    
} HscFDTDInterface;

/* ============================================================================
 * FDE-specific Interface
 * ============================================================================ */

/**
 * FDE solver configuration.
 */
typedef struct HscFDEConfig {
    uint32_t num_modes;        // Number of modes to compute
    double wavelength;         // Wavelength for mode calculation
    double bend_radius;        // Bend radius (0 for straight)
    double bend_angle;         // Bend angle in radians
    bool use_anti_pec;         // Use anti-PEC for symmetry
    double tolerance;          // Convergence tolerance
} HscFDEConfig;

/**
 * FDE-specific operations.
 */
typedef struct HscFDEInterface {
    // Base photonics interface
    HscPhotonicsInterface photonics;
    
    // FDE-specific operations
    HscErrorCode (*compute_modes)(
        HscPluginInstance* instance,
        uint32_t num_modes,
        double** effective_indices,
        HscComplex*** mode_fields
    );
    
    HscErrorCode (*get_mode_field)(
        HscPluginInstance* instance,
        uint32_t mode_index,
        HscFieldComponent component,
        HscComplex** field_data,
        HscIndex3* size
    );
    
    HscErrorCode (*calculate_overlap)(
        HscPluginInstance* instance,
        uint32_t mode1,
        uint32_t mode2,
        HscComplex* overlap
    );
    
    HscErrorCode (*calculate_group_index)(
        HscPluginInstance* instance,
        uint32_t mode_index,
        double* ng
    );
    
    HscErrorCode (*calculate_dispersion)(
        HscPluginInstance* instance,
        uint32_t mode_index,
        double* dispersion
    );
    
} HscFDEInterface;

/* ============================================================================
 * RCWA-specific Interface
 * ============================================================================ */

/**
 * RCWA solver configuration.
 */
typedef struct HscRCWAConfig {
    uint32_t num_orders;       // Number of diffraction orders
    uint32_t num_layers;       // Number of layers
    double* layer_thicknesses; // Layer thicknesses
    bool use_dispersive;       // Use dispersive materials
} HscRCWAConfig;

/**
 * RCWA-specific operations.
 */
typedef struct HscRCWAInterface {
    // Base photonics interface
    HscPhotonicsInterface photonics;
    
    // RCWA-specific operations
    HscErrorCode (*compute_diffraction)(
        HscPluginInstance* instance,
        double** r_orders,     // Reflection orders
        double** t_orders,     // Transmission orders
        uint32_t* num_orders
    );
    
    HscErrorCode (*get_efficiency)(
        HscPluginInstance* instance,
        int32_t order_m,
        int32_t order_n,
        double* r_efficiency,
        double* t_efficiency
    );
    
} HscRCWAInterface;

/* ============================================================================
 * Helper Functions
 * ============================================================================ */

/**
 * Create a default domain configuration.
 */
static inline HscDomain hsc_domain_default(void) {
    HscDomain domain = {
        .size = {1.0, 1.0, 0.5},
        .mesh = {100, 100, 50},
        .time_steps = 10000,
        .courant_factor = 0.99,
        .pml_layers = 8,
        .pml_sigma = 1.0,
    };
    for (int i = 0; i < 6; i++) {
        domain.boundaries[i] = HSC_BOUNDARY_PML;
    }
    return domain;
}

/**
 * Create a default FDTD configuration.
 */
static inline HscFDTDConfig hsc_fdtd_config_default(void) {
    return (HscFDTDConfig) {
        .courant_factor = 0.99,
        .auto_shutoff_min = 1e-5,
        .auto_shutoff_max = 1e5,
        .pml_layers = 8,
        .pml_sigma_max = 1.0,
        .use_dispersion = true,
        .use_subpixel = true,
        .dt = 0.0,
    };
}

/**
 * Create a default FDE configuration.
 */
static inline HscFDEConfig hsc_fde_config_default(void) {
    return (HscFDEConfig) {
        .num_modes = 10,
        .wavelength = 1.55,
        .bend_radius = 0.0,
        .bend_angle = 0.0,
        .use_anti_pec = false,
        .tolerance = 1e-6,
    };
}

/**
 * Create a default material (silica).
 */
static inline HscMaterial hsc_material_silica(void) {
    return (HscMaterial) {
        .name = "SiO2",
        .base_refractive_index = 1.45,
        .dispersion = HSC_DISP_SELLMEIER,
        .dispersion_params = {0.696, 0.069, 0.408, 0.116, 0.897, 9.896, 0, 0},
        .loss = 0.0,
        .is_anisotropic = false,
    };
}

/**
 * Create a default material (silicon).
 */
static inline HscMaterial hsc_material_silicon(void) {
    return (HscMaterial) {
        .name = "Si",
        .base_refractive_index = 3.48,
        .dispersion = HSC_DISP_POLYNOMIAL,
        .dispersion_params = {3.48, -0.001, 1e-6, 0, 0, 0, 0, 0},
        .loss = 0.0,
        .is_anisotropic = false,
    };
}

/**
 * Create a default Gaussian source.
 */
static inline HscSource hsc_source_gaussian_default(void) {
    return (HscSource) {
        .name = "gaussian_source",
        .region = {{0.45, 0.45, 0.0}, {0.55, 0.55, 0.0}},
        .wavelength = 1.55,
        .bandwidth = 0.1,
        .polarization = HSC_POL_TE,
        .amplitude = 1.0,
        .phase = 0.0,
        .time_profile = "gaussian",
        .pulse_width = 30.0,
    };
}

#ifdef __cplusplus
}
#endif

#endif /* HSC_PHOTONICS_INTERFACE_H */
