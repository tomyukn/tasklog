use crate::db::Database;
use crate::task::{Task, TaskTime, TimeDisplay};
use anyhow::Result;

pub fn run(db: &mut Database, time: Option<String>) -> Result<()> {
    // build end time
    let end_time = match time {
        Some(t) => TaskTime::parse_from_str_hhmm(&t)?,
        None => TaskTime::now(),
    };

    // fill end time of the current task
    if let Some(current_task_id) = db.get_current_task_id()? {
        let updated_task = fill_end_time(db, current_task_id, &end_time)?;
        db.reset_manager()?;

        println!(
            "{} ended at {}",
            &updated_task.name(),
            &updated_task.end_time().unwrap().to_string_hhmm()
        );
    }

    Ok(())
}

/// Fill the end time.
fn fill_end_time(db: &mut Database, task_id: u32, end_time: &TaskTime) -> Result<Task> {
    let mut task = db.get_task(task_id)?;
    task.set_end_time(Some(*end_time));
    db.update_task(task_id, &task)?;

    Ok(task)
}
