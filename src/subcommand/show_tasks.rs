use crate::db::Database;
use anyhow::Result;
use prettytable::{format, Table};

/// Remove a task name from the database.
pub fn run(db: &Database) -> Result<()> {
    let tasks = db.get_tasknames()?;
    print_tasks(tasks);

    Ok(())
}

/// Print task names as a table format.
fn print_tasks(tasks: Vec<(u32, String)>) {
    let mut table = Table::new();
    let table_format = format::FormatBuilder::new().padding(1, 1).build();
    table.set_format(table_format);

    // title
    table.add_row(row![br -> "No", bl -> "Task"]);

    // contents
    for (num, task_name) in tasks {
        table.add_row(row![r -> num, l -> task_name]);
    }

    table.printstd();
}
