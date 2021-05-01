use crate::db::Database;
use crate::task::{Task, TaskTime, TimeDisplay};
use anyhow::{anyhow, Result};

pub fn run(
    db: &mut Database,
    taskname_number: Option<u32>,
    is_break_time: bool,
    time: Option<String>,
    break_taskname: &str,
) -> Result<()> {
    let start_time = build_start_time(time, TaskTime::now())?;

    // end current task
    if let Some(current_task_id) = db.get_current_task_id()? {
        let updated_task = fill_end_time(db, current_task_id, &start_time)?;

        println!(
            "{} ended at {}",
            updated_task.name(),
            updated_task.end_time().unwrap().to_string_hhmm()
        );
    }

    // start new task
    let new_task_name = if is_break_time {
        String::from(break_taskname)
    } else {
        match taskname_number {
            Some(n) => db.get_taskname(n),
            None => Err(anyhow!("Task number was not provided")),
        }?
    };
    let new_task = register_task(db, new_task_name, start_time)?;

    println!(
        "{} started at {}",
        new_task.name(),
        new_task.start_time().to_string_hhmm()
    );

    Ok(())
}

/// Build `TaskTime` form `HHMM` string.
fn build_start_time(time: Option<String>, default: TaskTime) -> Result<TaskTime> {
    let start_time = match time {
        Some(t) => TaskTime::parse_from_str_hhmm(&t)?,
        None => default,
    };

    Ok(start_time)
}

/// Fill the end time.
fn fill_end_time(db: &mut Database, task_id: u32, end_time: &TaskTime) -> Result<Task> {
    let mut task = db.get_task(task_id)?;
    task.set_end_time(Some(*end_time));
    db.update_task(task_id, &task)?;

    Ok(task)
}

// start a task and register it to the database
fn register_task(db: &mut Database, task_name: String, start_time: TaskTime) -> Result<Task> {
    let new_task = Task::start(task_name, start_time);
    db.add_task_entry(&new_task)?;

    Ok(new_task)
}
