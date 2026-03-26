// YAML checklist registry — loads and hot-reloads checklist definitions.
// Why: Declarative checklist management via config files avoids hardcoding ops runbooks.
use std::collections::HashMap;
use std::path::Path;

use crate::checklist::engine::{Checklist, ChecklistError};

/// Intermediate YAML structure — maps serde_yaml fields to our domain types.
/// Why separate from Checklist: YAML uses lowercase strings ("critical"),
/// while our domain uses typed enums (CheckSeverity::Critical).
#[derive(serde::Deserialize)]
struct YamlChecklist {
    name: String,
    version: String,
    mode: String,
    items: Vec<YamlItem>,
}

#[derive(serde::Deserialize)]
struct YamlItem {
    id: String,
    title: String,
    command: String,
    expected: String,
    severity: String,
    #[serde(default)]
    depends_on: Vec<String>,
}

impl YamlChecklist {
    fn into_checklist(self) -> Result<Checklist, ChecklistError> {
        use crate::checklist::engine::{CheckItem, CheckMode, CheckSeverity};

        let mode = match self.mode.as_str() {
            "do-confirm" => CheckMode::DoConfirm,
            "read-do" => CheckMode::ReadDo,
            other => return Err(ChecklistError::Parse(format!("unknown mode: {other}"))),
        };

        let items = self
            .items
            .into_iter()
            .map(|yi| {
                let severity = match yi.severity.as_str() {
                    "critical" => CheckSeverity::Critical,
                    "warning" => CheckSeverity::Warning,
                    "info" => CheckSeverity::Info,
                    other => {
                        return Err(ChecklistError::Parse(format!(
                            "unknown severity: {other}"
                        )))
                    }
                };
                Ok(CheckItem {
                    id: yi.id,
                    title: yi.title,
                    command: yi.command,
                    expected: yi.expected,
                    severity,
                    depends_on: yi.depends_on,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Checklist {
            id: self.name.clone(),
            name: self.name,
            version: self.version,
            mode,
            items,
        })
    }
}

/// In-memory registry of checklists loaded from a directory of YAML files.
pub struct ChecklistRegistry {
    checklists: HashMap<String, Checklist>,
}

impl ChecklistRegistry {
    /// Scan `path` for `.yaml`/`.yml` files and parse each into a Checklist.
    pub fn load_directory(path: &Path) -> Result<Self, ChecklistError> {
        let mut checklists = HashMap::new();
        load_dir_into(path, &mut checklists)?;
        Ok(Self { checklists })
    }

    pub fn get(&self, name: &str) -> Option<&Checklist> {
        self.checklists.get(name)
    }

    pub fn list(&self) -> Vec<&Checklist> {
        self.checklists.values().collect()
    }

    /// Construct a registry from an in-memory collection (primarily for tests).
    pub fn from_checklists(checklists: Vec<Checklist>) -> Self {
        let map = checklists.into_iter().map(|c| (c.name.clone(), c)).collect();
        Self { checklists: map }
    }

    /// Re-scan the directory, replacing the entire registry.
    pub fn reload(&mut self, path: &Path) -> Result<(), ChecklistError> {
        let mut fresh = HashMap::new();
        load_dir_into(path, &mut fresh)?;
        self.checklists = fresh;
        Ok(())
    }
}

fn load_dir_into(
    path: &Path,
    map: &mut HashMap<String, Checklist>,
) -> Result<(), ChecklistError> {
    let entries = std::fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();
        let ext = file_path.extension().and_then(|e| e.to_str());
        if ext != Some("yaml") && ext != Some("yml") {
            continue;
        }
        let contents = std::fs::read_to_string(&file_path)?;
        let yaml: YamlChecklist = serde_yaml::from_str(&contents)
            .map_err(|e| ChecklistError::Parse(format!("{}: {e}", file_path.display())))?;
        let checklist = yaml.into_checklist()?;
        map.insert(checklist.name.clone(), checklist);
    }
    Ok(())
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
