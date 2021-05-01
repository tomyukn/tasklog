use crate::db::Database;
use anyhow::Result;

/// Register a task name with the database.
pub fn run(db: &mut Database, task_name: &str) -> Result<()> {
    if let Err(e) = db.register_taskname(task_name) {
        eprintln!("{}: {}", e, task_name);
    }

    Ok(())
}
