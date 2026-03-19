// Environment migration: sub-modules per toolchain/config area

pub mod brew;
pub mod macos;
pub mod repos;
pub mod runners;
pub mod shell;
pub mod vscode;

use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct EnvBundle {
    pub brewfile: Option<brew::Brewfile>,
    pub vscode_extensions: Option<Vec<String>>,
    pub vscode_settings: Option<String>,
    pub repos: Option<Vec<repos::RepoInfo>>,
    pub shell: Option<shell::ShellConfig>,
    pub runners: Option<Vec<runners::RunnerConfig>>,
}

/// Which modules to apply during import.
#[derive(Debug, Clone, Default)]
pub struct Selections {
    pub brew: bool,
    pub vscode: bool,
    pub repos: bool,
    pub shell: bool,
    pub macos: bool,
    pub runners: bool,
}

impl Selections {
    pub fn all() -> Self {
        Self {
            brew: true,
            vscode: true,
            repos: true,
            shell: true,
            macos: true,
            runners: true,
        }
    }
}

/// Exports everything available on the current machine.
pub fn export_all(github_dir: &Path, runner_paths: &[String]) -> EnvBundle {
    let brewfile = brew::export_brewfile().ok();

    let vscode_extensions = vscode::export_extensions().ok();
    let vscode_settings = vscode::export_settings();

    let repos = if github_dir.exists() {
        Some(repos::scan_github_dir(github_dir))
    } else {
        None
    };

    let shell = shell::export_shell_config().ok();

    let runners = if !runner_paths.is_empty() {
        let found = runners::scan_runners(runner_paths);
        if found.is_empty() {
            None
        } else {
            Some(found)
        }
    } else {
        None
    };

    EnvBundle {
        brewfile,
        vscode_extensions,
        vscode_settings,
        repos,
        shell,
        runners,
    }
}

/// Applies selected parts of an `EnvBundle` to the current machine.
pub fn import_all(
    bundle: &EnvBundle,
    selections: &Selections,
    home: &Path,
    clone_target: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if selections.brew {
        if let Some(ref bf) = bundle.brewfile {
            let all: Vec<String> = bf.formulae.keys().cloned().collect();
            brew::install_brewfile(bf, &all).ok();
        }
    }

    if selections.vscode {
        if let Some(ref exts) = bundle.vscode_extensions {
            vscode::import_extensions(exts).ok();
        }
        if let Some(ref settings) = bundle.vscode_settings {
            vscode::import_settings(settings, home).ok();
        }
    }

    if selections.repos {
        if let Some(ref repo_list) = bundle.repos {
            if let Some(target) = clone_target {
                repos::clone_repos(repo_list, target).ok();
            }
        }
    }

    if selections.shell {
        if let Some(ref cfg) = bundle.shell {
            shell::import_shell_config(cfg, home).ok();
        }
    }

    if selections.macos {
        macos::apply_all().ok();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selections_all() {
        let sel = Selections::all();
        assert!(sel.brew && sel.vscode && sel.repos && sel.shell && sel.macos && sel.runners);
    }

    #[test]
    fn test_env_bundle_default() {
        let bundle = EnvBundle::default();
        assert!(bundle.brewfile.is_none());
        assert!(bundle.repos.is_none());
    }

    #[test]
    fn test_export_all_nonexistent_github_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let github_dir = tmp.path().join("nonexistent");
        let bundle = export_all(&github_dir, &[]);
        assert!(bundle.repos.is_none());
    }

    #[test]
    fn test_export_all_empty_github_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bundle = export_all(tmp.path(), &[]);
        // Empty dir → Some([]) from scan
        assert!(bundle.repos.is_some());
        assert_eq!(bundle.repos.unwrap().len(), 0);
    }
}
