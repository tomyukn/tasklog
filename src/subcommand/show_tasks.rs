use crate::db::Database;
use anyhow::Result;
use prettytable::{format, Table};

/// Remove a task name from the database.
pub fn run(db: &Database) -> Result<()> {
    let tasknames = db.get_registered_tasknames()?;
    print_tasknames(tasknames);

    Ok(())
}

/// Print task names as a table format.
fn print_tasknames(tasknames: Vec<(u32, String)>) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // title
    table.add_row(row![br -> "No", bl -> "Task"]);

    // contents
    for (num, task_name) in tasknames {
        table.add_row(row![r -> num, l -> task_name]);
    }

    table.printstd();
}
