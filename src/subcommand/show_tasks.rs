use crate::db::Database;
use anyhow::Result;
use std::path::PathBuf;

/// Remove a task name from the database.
pub fn run(db_path: PathBuf) -> Result<()> {
    let db = Database::connect_r(&db_path)?;

    let tasks = db.get_tasknames()?;
    for (num, task_name) in tasks {
        println!("{0:>2}. {1}", num, task_name);
    }

    Ok(())
}
