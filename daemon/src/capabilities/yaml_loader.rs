use super::registry::CapabilityRegistry;
use super::types::{Capability, CapabilityError};
use std::fs;
use std::path::Path;

/// Load capability definitions from YAML files in a directory.
/// Each .yaml file defines one or more capabilities.
pub fn load_from_dir(
    dir: &Path,
    registry: &CapabilityRegistry,
) -> Result<usize, CapabilityError> {
    if !dir.exists() {
        return Ok(0);
    }
    let entries = fs::read_dir(dir)
        .map_err(|e| CapabilityError::InvocationFailed(format!("read dir: {e}")))?;
    let mut count = 0;
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yaml" && ext != "yml" {
            continue;
        }
        let loaded = load_file(&path, registry)?;
        count += loaded;
    }
    Ok(count)
}

/// Load capabilities from a single YAML file.
pub fn load_file(
    path: &Path,
    registry: &CapabilityRegistry,
) -> Result<usize, CapabilityError> {
    let content = fs::read_to_string(path)
        .map_err(|e| CapabilityError::InvocationFailed(format!("read {}: {e}", path.display())))?;
    let caps: Vec<CapabilityDef> = serde_yaml::from_str(&content)
        .map_err(|e| CapabilityError::InvalidInput(format!("parse {}: {e}", path.display())))?;
    let mut count = 0;
    for def in caps {
        let cap = Capability {
            name: def.name,
            description: def.description,
            ring: def.ring,
            mcp_server: def.mcp_server,
            input_schema: def.input_schema.unwrap_or(serde_json::json!({})),
            permissions_required: def.permissions_required.unwrap_or_default(),
            enabled: def.enabled.unwrap_or(true),
        };
        registry.register(cap)?;
        count += 1;
    }
    Ok(count)
}

/// YAML capability definition.
#[derive(serde::Deserialize)]
struct CapabilityDef {
    name: String,
    description: String,
    ring: u8,
    mcp_server: Option<String>,
    input_schema: Option<serde_json::Value>,
    permissions_required: Option<Vec<String>>,
    enabled: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_from_yaml_file() {
        let dir = TempDir::new().unwrap();
        let yaml = r#"
- name: read-file
  description: Read a file from disk
  ring: 0
  input_schema:
    type: object
    properties:
      path:
        type: string
- name: stripe-charge
  description: Create a Stripe charge
  ring: 2
  mcp_server: "stdio://stripe-server"
  permissions_required:
    - stripe:write
"#;
        fs::write(dir.path().join("tools.yaml"), yaml).unwrap();
        let reg = CapabilityRegistry::new();
        let loaded = load_from_dir(dir.path(), &reg).unwrap();
        assert_eq!(loaded, 2);
        assert_eq!(reg.count(), 2);
        let rf = reg.get("read-file").unwrap();
        assert_eq!(rf.ring, 0);
        let sc = reg.get("stripe-charge").unwrap();
        assert!(sc.mcp_server.is_some());
    }

    #[test]
    fn empty_dir_returns_zero() {
        let dir = TempDir::new().unwrap();
        let reg = CapabilityRegistry::new();
        assert_eq!(load_from_dir(dir.path(), &reg).unwrap(), 0);
    }

    #[test]
    fn nonexistent_dir_returns_zero() {
        let reg = CapabilityRegistry::new();
        assert_eq!(load_from_dir(Path::new("/nonexistent"), &reg).unwrap(), 0);
    }
}
