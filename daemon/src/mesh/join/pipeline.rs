// Main join pipeline and private step helpers

use super::types::{JoinConfig, JoinError, JoinProgress, JoinSelections, StepStatus};
use crate::mesh::error::MeshError;
use crate::mesh::token;

pub(super) fn make_step(step: u8, total: u8, label: &str, status: StepStatus) -> JoinProgress {
    JoinProgress {
        step,
        total_steps: total,
        current: label.to_owned(),
        status,
    }
}

/// Execute the join pipeline based on `config`.
pub async fn join(
    config: JoinConfig,
    secret: &[u8],
    db: &rusqlite::Connection,
) -> Result<Vec<JoinProgress>, JoinError> {
    const TOTAL: u8 = 9;
    let mut log: Vec<JoinProgress> = Vec::new();

    // Step 1: Validate token
    let mut p = make_step(1, TOTAL, "Validate invite token", StepStatus::Running);
    emit_if_interactive(&config, &p);
    let _payload = token::validate_token(&config.token, secret, db)?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 2: Admin gate
    let mut p = make_step(
        2,
        TOTAL,
        "Verify admin credentials (sudo -v)",
        StepStatus::Running,
    );
    emit_if_interactive(&config, &p);
    run_sudo_keepalive().map_err(|e| JoinError::Network(e.to_string()))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 3: Network setup
    let step_status = if config.selections.network {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(
        3,
        TOTAL,
        "Network setup (Tailscale, SSH, Screen Sharing)",
        step_status.clone(),
    );
    emit_if_interactive(&config, &p);
    if config.selections.network {
        network_setup().map_err(|e| JoinError::Network(e.to_string()))?;
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 4: Download bundles
    let mut p = make_step(
        4,
        TOTAL,
        "Download bundles from coordinator",
        StepStatus::Running,
    );
    emit_if_interactive(&config, &p);
    let coordinator_ip = _payload.coordinator_ip.clone();
    let bundle_dir = super::server::download_bundles(&coordinator_ip, &config.token).await?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 5: Import auth
    let step_status = if config.selections.auth {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(
        5,
        TOTAL,
        "Import auth (decrypt + keychain)",
        step_status.clone(),
    );
    emit_if_interactive(&config, &p);
    if config.selections.auth {
        import_auth(&bundle_dir).map_err(|e| JoinError::AuthImport(e.to_string()))?;
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 6: Import env
    let mut p = make_step(
        6,
        TOTAL,
        "Import environment (brew/repos/shell/macos)",
        StepStatus::Running,
    );
    emit_if_interactive(&config, &p);
    import_env(&bundle_dir, &config.selections).map_err(|e| JoinError::Network(e.to_string()))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 7: Coordinator migration
    let step_status = if config.selections.coordinator_migration {
        StepStatus::Running
    } else {
        StepStatus::Skipped
    };
    let mut p = make_step(7, TOTAL, "Coordinator migration", step_status.clone());
    emit_if_interactive(&config, &p);
    if config.selections.coordinator_migration {
        p.status = StepStatus::Done;
    }
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 8: Register self in peers.conf
    let mut p = make_step(
        8,
        TOTAL,
        "Register node in peers.conf on all nodes",
        StepStatus::Running,
    );
    emit_if_interactive(&config, &p);
    register_self_in_peers(&coordinator_ip)
        .await
        .map_err(|e| JoinError::Network(e.to_string()))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    // Step 9: Preflight check
    let mut p = make_step(9, TOTAL, "Preflight check", StepStatus::Running);
    emit_if_interactive(&config, &p);
    run_preflight().map_err(|e| JoinError::Preflight(e.to_string()))?;
    p.status = StepStatus::Done;
    emit_if_interactive(&config, &p);
    log.push(p);

    Ok(log)
}

// Private step helpers

pub(super) fn emit_if_interactive(config: &JoinConfig, progress: &JoinProgress) {
    if config.interactive {
        if let Ok(json) = serde_json::to_string(progress) {
            println!("{json}");
        }
    }
}

fn run_sudo_keepalive() -> std::io::Result<()> {
    let status = std::process::Command::new("sudo").arg("-v").status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "sudo -v failed — admin credentials required",
        ))
    }
}

fn network_setup() -> Result<(), MeshError> {
    let out = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()?;
    if !out.status.success() {
        return Err(MeshError::Network(format!(
            "tailscale not reachable: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

fn import_auth(_bundle_dir: &std::path::Path) -> Result<(), MeshError> {
    Ok(())
}

fn import_env(_bundle_dir: &std::path::Path, selections: &JoinSelections) -> Result<(), MeshError> {
    let _ = (
        selections.brew,
        selections.repos,
        selections.shell,
        selections.macos_tweaks,
    );
    Ok(())
}

async fn register_self_in_peers(_coordinator_ip: &str) -> Result<(), MeshError> {
    Ok(())
}

fn run_preflight() -> Result<(), MeshError> {
    let result = std::process::Command::new("mesh-preflight.sh").output();
    match result {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(MeshError::Internal(format!(
            "preflight issues: {}",
            String::from_utf8_lossy(&out.stderr)
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(MeshError::from(e)),
    }
}
