pub mod registry;
pub mod ring;
pub mod types;

pub use ring::Ring;
pub use types::{Capability, CapabilityError, ToolSchema};

/// Core trait for executable capabilities.
/// Every tool (local or MCP) implements this to be invokable by agents.
pub trait CapabilityProvider: Send + Sync {
    fn name(&self) -> &str;
    fn describe(&self) -> ToolSchema;
    fn ring_level(&self) -> Ring;
    fn validate_input(&self, input: &serde_json::Value) -> Result<(), CapabilityError>;
    fn invoke(&self, input: serde_json::Value) -> Result<serde_json::Value, CapabilityError>;
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;

#[cfg(test)]
#[path = "ring_tests.rs"]
mod ring_tests;

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;
