use crate::db::Database;
use crate::task::{Task, TaskTime, TimeDisplay};
use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn run(
    db_path: PathBuf,
    task_number: Option<u32>,
    is_break_time: bool,
    time: Option<String>,
    break_taskname: &str,
) -> Result<()> {
    // build start time
    let start_time = match time {
        Some(t) => TaskTime::parse_from_str_hhmm(&t)?,
        None => TaskTime::now(),
    };

    let mut db = Database::connect_rw(&db_path)?;

    // fill end time of the previous task if it is empty
    if let Some(current_task_id) = db.get_current_task_id()? {
        let mut current_task = db.get_task(current_task_id)?;

        let updated_task = current_task.set_end_time(Some(start_time));
        db.update_task(current_task_id, &updated_task)?;

        println!(
            "{} ended at {}",
            &updated_task.name(),
            &start_time.to_string_hhmm()
        );
    }

    // create a new task
    let new_task_name = if is_break_time {
        Ok(String::from(break_taskname))
    } else {
        match task_number {
            Some(id) => db.get_taskname(id),
            None => Err(anyhow!("Task number was not provided")),
        }
    }?;

    let new_task = Task::start(new_task_name.clone(), start_time);
    db.add_task_entry(&new_task)?;

    println!(
        "{} started at {}",
        new_task_name,
        start_time.to_string_hhmm()
    );

    Ok(())
}
