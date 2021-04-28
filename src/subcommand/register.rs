use crate::db::Database;
use anyhow::Result;
use std::path::PathBuf;

/// Register a task name with the database.
pub fn run(db_path: PathBuf, task_name: &str) -> Result<()> {
    let mut db = Database::connect_rw(&db_path)?;

    if let Err(e) = db.register_taskname(task_name) {
        println!("{}: {}", e, task_name);
    }

    Ok(())
}
