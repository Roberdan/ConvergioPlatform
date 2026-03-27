pub mod blueprint;
pub mod registry;
pub mod renderer;
pub mod scanner;
pub mod types;

pub use registry::ArtifactRegistry;
pub use types::{Artifact, ArtifactError, ArtifactType, Maturity};

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "scanner_tests.rs"]
mod scanner_tests;
