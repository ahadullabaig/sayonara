// Drive operations and management
//
// This module provides drive-level operations that work across different drive types

pub mod hpa_dco;   // Hidden Protected Area / Device Configuration Overlay
pub mod sed;       // Self-Encrypting Drive operations
pub mod trim;      // TRIM/discard operations
pub mod smart;     // SMART monitoring and health checks

// Re-exports for convenience
pub use hpa_dco::HPADCOManager;
pub use sed::SEDManager;
pub use trim::TrimOperations;
pub use smart::SMARTMonitor;

// Tests
#[cfg(test)]
mod smart_tests;

#[cfg(test)]
mod hpa_dco_tests;

#[cfg(test)]
mod trim_tests;
