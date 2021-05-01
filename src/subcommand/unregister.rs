use crate::db::Database;
use anyhow::Result;

/// Remove a task name from the database.
pub fn run(db: &mut Database, task_name: &str) -> Result<()> {
    if let Err(e) = db.unregister_taskname(task_name) {
        eprintln!("{}: {}", e, task_name);
    }

    Ok(())
}
