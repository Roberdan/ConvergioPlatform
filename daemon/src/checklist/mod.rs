pub mod api;
pub mod cli;
pub mod engine;
pub mod registry;
pub mod runners;
pub mod telemetry;
pub mod thor_gate;

#[cfg(test)]
mod engine_tests;

#[cfg(test)]
#[path = "registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "nasa_rules_tests.rs"]
mod nasa_rules_tests;

#[cfg(test)]
#[path = "thor_gate_tests.rs"]
mod thor_gate_tests;

#[cfg(test)]
#[path = "thor_gate_validate_all_tests.rs"]
mod thor_gate_validate_all_tests;
