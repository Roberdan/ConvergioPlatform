// Unified error type re-exports for the daemon.
// Import from here instead of reaching into individual module paths.

pub use crate::mesh::coordinator::CoordinatorError;
pub use crate::mesh::error::MeshError;
pub use crate::mesh::join::JoinError;
