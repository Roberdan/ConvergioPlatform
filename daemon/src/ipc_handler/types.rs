use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum IpcCommands {
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    Models {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Sub {
        #[command(subcommand)]
        command: SubCommands,
    },
    Budget {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Route {
        task_description: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        parallel: bool,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Skills {
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RequestSkill {
        skill: String,
        #[arg(long)]
        payload: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RespondSkill {
        request_id: String,
        #[arg(long)]
        result: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    RateSkill {
        request_id: String,
        rating: f64,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    Store {
        service: String,
        token: String,
        #[arg(long, env = "IPC_SHARED_SECRET")]
        secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Get {
        service: String,
        #[arg(long, env = "IPC_SHARED_SECRET")]
        secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Revoke {
        service: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Rotate {
        #[arg(long, env = "IPC_OLD_SECRET")]
        old_secret: String,
        #[arg(long, env = "IPC_NEW_SECRET")]
        new_secret: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SubCommands {
    Add {
        name: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        plan: String,
        #[arg(long)]
        budget: f64,
        #[arg(long, default_value_t = 1)]
        reset_day: i32,
        #[arg(long, value_delimiter = ',')]
        models: Vec<String>,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
    Remove {
        name: String,
        #[arg(long)]
        db_path: Option<PathBuf>,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum DaemonCommands {
    Start {
        #[arg(long)]
        bind_ip: Option<String>,
        #[arg(long, default_value_t = 9420)]
        port: u16,
        #[arg(long)]
        peers_conf: Option<PathBuf>,
        #[arg(long)]
        db_path: Option<PathBuf>,
        #[arg(long)]
        crsqlite_path: Option<String>,
        #[arg(long, default_value_t = false)]
        local_only: bool,
    },
}
