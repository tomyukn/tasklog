use crate::db::Database;
use anyhow::Result;
use std::path::PathBuf;

/// Remove a task name from the database.
pub fn run(db_path: PathBuf, task_name: &str) -> Result<()> {
    let mut db = Database::connect_rw(&db_path)?;

    if let Err(e) = db.unregister_taskname(task_name) {
        eprintln!("{}: {}", e, task_name);
    }

    Ok(())
}
