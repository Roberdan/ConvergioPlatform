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
    let brewfile = match brew::export_brewfile() {
        Ok(bf) => Some(bf),
        Err(e) => {
            eprintln!("WARN: brew export failed: {e}");
            None
        }
    };

    let vscode_extensions = match vscode::export_extensions() {
        Ok(exts) => Some(exts),
        Err(e) => {
            eprintln!("WARN: vscode extensions export failed: {e}");
            None
        }
    };
    let vscode_settings = vscode::export_settings();

    let repos = if github_dir.exists() {
        Some(repos::scan_github_dir(github_dir))
    } else {
        None
    };

    let shell = match shell::export_shell_config() {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("WARN: shell config export failed: {e}");
            None
        }
    };

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
            if let Err(e) = brew::install_brewfile(bf, &all) {
                eprintln!("WARN: brew install failed: {e}");
            }
        }
    }

    if selections.vscode {
        if let Some(ref exts) = bundle.vscode_extensions {
            if let Err(e) = vscode::import_extensions(exts) {
                eprintln!("WARN: vscode extensions import failed: {e}");
            }
        }
        if let Some(ref settings) = bundle.vscode_settings {
            if let Err(e) = vscode::import_settings(settings, home) {
                eprintln!("WARN: vscode settings import failed: {e}");
            }
        }
    }

    if selections.repos {
        if let Some(ref repo_list) = bundle.repos {
            if let Some(target) = clone_target {
                if let Err(e) = repos::clone_repos(repo_list, target) {
                    eprintln!("WARN: repos clone failed: {e}");
                }
            }
        }
    }

    if selections.shell {
        if let Some(ref cfg) = bundle.shell {
            if let Err(e) = shell::import_shell_config(cfg, home) {
                eprintln!("WARN: shell config import failed: {e}");
            }
        }
    }

    if selections.macos {
        if let Err(e) = macos::apply_all() {
            eprintln!("WARN: macos defaults apply failed: {e}");
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "../env_tests.rs"]
mod tests;
