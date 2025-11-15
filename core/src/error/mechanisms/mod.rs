/// Recovery mechanisms for error handling
///
/// This module provides various recovery mechanisms that can be used
/// to handle different types of failures during wipe operations.
pub mod alternative_io;
pub mod bad_sector;
pub mod degraded_mode;
pub mod self_heal;

// Re-export main types
pub use alternative_io::{AlternativeIO, IOMethod};
pub use bad_sector::{BadSectorHandler, BadSectorReport, WriteResult};
pub use degraded_mode::{DegradedMode, DegradedModeManager};
pub use self_heal::{HealMethod, SelfHealer};
