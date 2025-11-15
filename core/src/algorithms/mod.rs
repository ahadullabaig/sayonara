pub mod dod;
pub mod gutmann;
pub mod random;
pub mod zero;

#[cfg(test)]
mod dod_test;
#[cfg(test)]
mod functional_tests;
#[cfg(test)]
mod gutmann_test;
#[cfg(test)]
mod random_test;

// Re-export the main wiping implementations
pub use dod::DoDWipe;
pub use gutmann::GutmannWipe;
pub use random::RandomWipe;
pub use zero::ZeroWipe;
