// Row-mapping helpers and error constructor extracted from db/service.rs.
// Why: keep service.rs ≤250 lines per CONSTITUTION Article V.
use super::{ActivePlan, InProgressTask};
use std::io::{Error as IoError, ErrorKind};

pub(crate) fn map_active_plan(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActivePlan> {
    Ok(ActivePlan {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        status: row.get(3)?,
        tasks_done: row.get(4)?,
        tasks_total: row.get(5)?,
    })
}

pub(crate) fn map_in_progress_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<InProgressTask> {
    Ok(InProgressTask {
        id: row.get(0)?,
        project_id: row.get(1)?,
        task_id: row.get(2)?,
        title: row.get(3)?,
        wave_id: row.get(4)?,
    })
}

pub(crate) fn invalid_input(message: &str) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(IoError::new(
        ErrorKind::InvalidInput,
        message.to_string(),
    )))
}
