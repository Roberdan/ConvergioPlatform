use super::types::SecurityError;
use std::collections::HashMap;
use std::sync::RwLock;

/// Secret storage abstraction over macOS Keychain.
/// In production: uses Security.framework via security-framework crate.
/// In test/non-macOS: uses in-memory store.
pub struct SecretStore {
    /// Service name for Keychain entries.
    service: String,
    /// In-memory fallback for testing and non-macOS.
    memory_store: RwLock<HashMap<String, String>>,
}

impl SecretStore {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
            memory_store: RwLock::new(HashMap::new()),
        }
    }

    /// Store a secret (API key, token, etc.).
    pub fn store(&self, account: &str, secret: &str) -> Result<(), SecurityError> {
        #[cfg(target_os = "macos")]
        {
            self.store_keychain(account, secret)?;
        }
        // Always store in memory for fallback/test.
        let mut store = self.memory_store.write()
            .map_err(|e| SecurityError::KeychainError(format!("lock: {e}")))?;
        store.insert(format!("{}:{}", self.service, account), secret.to_string());
        Ok(())
    }

    /// Retrieve a secret by account name.
    pub fn retrieve(&self, account: &str) -> Result<String, SecurityError> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(val) = self.retrieve_keychain(account) {
                return Ok(val);
            }
        }
        let store = self.memory_store.read()
            .map_err(|e| SecurityError::KeychainError(format!("lock: {e}")))?;
        let key = format!("{}:{}", self.service, account);
        store
            .get(&key)
            .cloned()
            .ok_or_else(|| SecurityError::KeychainError(format!("secret not found: {account}")))
    }

    /// Delete a secret.
    pub fn delete(&self, account: &str) -> Result<(), SecurityError> {
        let mut store = self.memory_store.write()
            .map_err(|e| SecurityError::KeychainError(format!("lock: {e}")))?;
        store.remove(&format!("{}:{}", self.service, account));
        Ok(())
    }

    /// List all stored account names (not secrets).
    pub fn list_accounts(&self) -> Vec<String> {
        let prefix = format!("{}:", self.service);
        self.memory_store
            .read()
            .map(|s| {
                s.keys()
                    .filter_map(|k| k.strip_prefix(&prefix).map(|a| a.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[cfg(target_os = "macos")]
    fn store_keychain(&self, account: &str, secret: &str) -> Result<(), SecurityError> {
        use std::process::Command;
        let status = Command::new("security")
            .args(["add-generic-password", "-a", account, "-s", &self.service,
                   "-w", secret, "-U"])
            .status()
            .map_err(|e| SecurityError::KeychainError(format!("security cmd: {e}")))?;
        if status.success() { Ok(()) } else {
            Err(SecurityError::KeychainError("keychain store failed".to_string()))
        }
    }

    #[cfg(target_os = "macos")]
    fn retrieve_keychain(&self, account: &str) -> Result<String, SecurityError> {
        use std::process::Command;
        let output = Command::new("security")
            .args(["find-generic-password", "-a", account, "-s", &self.service, "-w"])
            .output()
            .map_err(|e| SecurityError::KeychainError(format!("security cmd: {e}")))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(SecurityError::KeychainError("not found in keychain".to_string()))
        }
    }
}
