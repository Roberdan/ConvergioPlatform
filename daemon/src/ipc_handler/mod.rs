mod auth;
mod models_handler;
mod routing;
mod server;
mod types;
mod utils;

// Re-export public API — callers in main.rs use these
pub use server::{run_daemon, run_serve};
pub use types::{DaemonCommands, IpcCommands};
pub use utils::default_db_path;

pub async fn handle_ipc(command: IpcCommands) {
    match command {
        IpcCommands::Auth { command } => auth::handle_auth(command).await,
        IpcCommands::Models { db_path } => models_handler::handle_models(db_path),
        IpcCommands::Sub { command } => models_handler::handle_sub(command),
        IpcCommands::Budget { db_path } => models_handler::handle_budget(db_path),
        IpcCommands::Route {
            task_description,
            dry_run,
            parallel,
            db_path,
        } => routing::handle_route(task_description, dry_run, parallel, db_path),
        IpcCommands::Skills { agent, db_path } => routing::handle_skills(agent, db_path),
        IpcCommands::RequestSkill {
            skill,
            payload,
            db_path,
        } => routing::handle_request_skill(skill, payload, db_path),
        IpcCommands::RespondSkill {
            request_id,
            result,
            db_path,
        } => routing::handle_respond_skill(request_id, result, db_path),
        IpcCommands::RateSkill {
            request_id,
            rating,
            db_path,
        } => routing::handle_rate_skill(request_id, rating, db_path),
    }
}
