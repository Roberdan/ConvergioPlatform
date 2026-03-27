use serde::{Deserialize, Serialize};

/// Security ring for capability access control.
/// Lower ring number = more privilege (kernel-style).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Ring {
    /// Ring 0: Core platform tools — unrestricted access.
    Core = 0,
    /// Ring 1: Verified trusted tools — full I/O, no sandbox.
    Trusted = 1,
    /// Ring 2: Community tools — sandboxed with limited I/O.
    Community = 2,
    /// Ring 3: External/sandboxed tools — no filesystem, filtered network.
    Sandboxed = 3,
}

impl Ring {
    /// Convert from u8, defaulting to Sandboxed for unknown values.
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Ring::Core,
            1 => Ring::Trusted,
            2 => Ring::Community,
            _ => Ring::Sandboxed,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Whether this ring can access a capability at the given ring level.
    /// An agent at ring N can access capabilities at ring N or higher (less privileged).
    pub fn can_access(self, capability_ring: Ring) -> bool {
        (self as u8) <= (capability_ring as u8)
    }

    pub fn label(self) -> &'static str {
        match self {
            Ring::Core => "Core",
            Ring::Trusted => "Trusted",
            Ring::Community => "Community",
            Ring::Sandboxed => "Sandboxed",
        }
    }
}

impl std::fmt::Display for Ring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ring {} ({})", self.as_u8(), self.label())
    }
}
