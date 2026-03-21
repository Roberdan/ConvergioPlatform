use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "convergiomesh-cli", version, about = "Mesh node onboarding and environment migration tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate an invite token
    Invite {
        #[arg(long, default_value = "worker")]
        role: String,
        #[arg(long, default_value_t = 30)]
        ttl: u64,
        #[arg(long)]
        qr: bool,
    },
    /// Join mesh with invite token
    Join {
        token: String,
        #[arg(long)]
        password: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long, value_delimiter = ',')]
        profiles: Vec<String>,
    },
    /// Revoke an invite token by nonce
    Revoke {
        nonce: String,
    },
    /// Peer management
    Peers {
        #[command(subcommand)]
        action: PeersAction,
    },
    /// Network status
    Network {
        #[command(subcommand)]
        action: NetworkAction,
    },
    /// Auth credential management
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Environment management
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
    /// Coordinator management
    Coordinator {
        #[command(subcommand)]
        action: CoordinatorAction,
    },
    /// Package for USB/distribution
    Package {
        #[arg(long)]
        usb: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum PeersAction {
    /// List active peers
    List,
    /// Add a peer (config_json must be a JSON PeerConfig)
    Add { name: String, config_json: String },
    /// Remove a peer by name
    Remove { name: String },
}

#[derive(Subcommand)]
pub enum NetworkAction {
    /// Show Tailscale status
    Status,
}

#[derive(Subcommand)]
pub enum AuthAction {
    /// Export credentials to encrypted bundle
    Export {
        #[arg(long)]
        output: String,
    },
    /// Import credentials from encrypted bundle
    Import {
        #[arg(long)]
        bundle: String,
    },
}

#[derive(Subcommand)]
pub enum EnvAction {
    /// Export environment bundle to file
    Export {
        #[arg(long)]
        output: String,
    },
    /// Import environment bundle from file
    Import {
        #[arg(long)]
        bundle: String,
        #[arg(long)]
        profile: Option<String>,
    },
    /// List available profiles
    ListProfiles,
}

#[derive(Subcommand)]
pub enum CoordinatorAction {
    /// Migrate coordinator role to a new node
    Migrate {
        #[arg(long)]
        to: String,
    },
    /// Rollback coordinator migration from a snapshot file
    Rollback {
        #[arg(long)]
        snapshot: String,
    },
    /// Show coordinator migration status
    Status,
}
