// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Centralized platform path resolution (F-21).
// Provides OS-aware data directories with fallback to ~/.convergio/.

use std::path::PathBuf;

/// Primary Convergio data directory.
/// macOS: ~/Library/Application Support/Convergio
/// Linux: ~/.local/share/convergio
/// Windows: %APPDATA%/Convergio
/// Fallback: ~/.convergio/
pub fn convergio_data_dir() -> PathBuf {
    if let Some(data) = dirs::data_dir() {
        return data.join("Convergio");
    }
    // Fallback when dirs crate cannot resolve platform dir
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".convergio")
}

/// Output directory for a named project: <data_dir>/projects/<name>/output/
pub fn project_output_dir(project_name: &str) -> PathBuf {
    convergio_data_dir()
        .join("projects")
        .join(project_name)
        .join("output")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_is_absolute_or_fallback() {
        let dir = convergio_data_dir();
        // Must end with "Convergio" or ".convergio"
        let name = dir.file_name().unwrap().to_str().unwrap();
        assert!(
            name == "Convergio" || name == ".convergio",
            "unexpected dir name: {name}"
        );
    }

    #[test]
    fn project_output_dir_structure() {
        let out = project_output_dir("my-app");
        assert!(out.ends_with("projects/my-app/output"));
    }
}
