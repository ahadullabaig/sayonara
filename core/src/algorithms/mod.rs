pub mod gutmann;
pub mod dod;
pub mod random;
pub mod zero;

#[cfg(test)]
mod gutmann_test;
#[cfg(test)]
mod dod_test;
#[cfg(test)]
mod random_test;
#[cfg(test)]
mod functional_tests;

// Re-export the main wiping implementations
pub use dod::DoDWipe;
pub use gutmann::GutmannWipe;
pub use random::RandomWipe;
pub use zero::ZeroWipe;
