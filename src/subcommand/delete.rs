use crate::db::Database;
use crate::task::{TimeDisplay, WorkDate};
use anyhow::Result;
use dialoguer::Confirm;

pub fn run(db: &mut Database, task_number: u32) -> Result<()> {
    let working_date = WorkDate::now();

    let task_id = db.get_task_id_by_seqnum(task_number, working_date)?;
    let task = db.get_task(task_id)?;

    eprintln!(
        "\"{}\" started at {} {}",
        task.name(),
        task.working_date().to_string(),
        task.start_time().to_string_hhmm()
    );

    let proceed = Confirm::new()
        .with_prompt("Really delete?")
        .wait_for_newline(false)
        .default(false)
        .show_default(true)
        .interact()?;
    if proceed {
        db.delete_task(task_id)?;
        println!("\ntask {} deleted", task_number);
    } else {
        println!("\nOparation canceled.");
    };

    Ok(())
}
