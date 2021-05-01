use crate::db::Database;
use crate::task::{TaskTime, WorkDate};
use anyhow::Result;

pub fn run(db: &Database, task_number: u32, target: String, value: String) -> Result<()> {
    let working_date = WorkDate::now();

    let task_id = db.get_task_id_by_seqnum(task_number, working_date)?;
    let mut task = db.get_task(task_id)?;

    if target == "name" {
        task.set_name(value);
    } else if target == "start" {
        task.set_start_time(TaskTime::parse_from_str_hhmm(&value)?);
    } else if target == "end" {
        task.set_end_time(Some(TaskTime::parse_from_str_hhmm(&value)?));
    }

    db.update_task(task.id().unwrap(), &task)
}
