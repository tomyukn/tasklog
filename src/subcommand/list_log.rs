use crate::db::Database;
use crate::task::{Task, TaskList, TaskSummary, TimeDisplay, WorkDate};
use anyhow::Result;
use prettytable::{format, table, Row, Table};
use std::path::PathBuf;

/// Print task log
pub fn run(
    db_path: PathBuf,
    show_all: bool,
    date: Option<String>,
    break_taskname: &str,
) -> Result<()> {
    let db = Database::connect_rw(&db_path)?;

    let date = build_date(date, WorkDate::now())?;
    let tasks_with_seq = db.get_tasks(show_all, Some(date))?;

    // show list
    print_list(tasks_with_seq.clone())?;

    // show details
    let mut tasks: Vec<Task> = Vec::new();
    let mut breaks: Vec<Task> = Vec::new();

    for (_, task) in tasks_with_seq {
        if task.name() == break_taskname {
            breaks.push(task);
        } else {
            tasks.push(task);
        };
    }

    // show summary
    if !show_all {
        if let Some(task_summary) = TaskList::new(tasks).summary() {
            println!("");
            print_summary(task_summary, breaks)?;
        }
    }

    Ok(())
}

/// Build `WorkDate` form an input string.
fn build_date(date: Option<String>, default: WorkDate) -> Result<WorkDate> {
    let date = match date {
        Some(s) => WorkDate::parse_from_str(&s)?,
        None => default,
    };

    Ok(date)
}

// Print task log
fn print_list(task_list: Vec<(u32, Task)>) -> Result<()> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // title
    table.set_titles(row![b => "Date", "No", "Start", "End", "Duration", "Task"]);

    // contents
    for (n, task) in task_list {
        let date = task.working_date().to_string();
        let start = task.start_time().to_string_hhmm();
        let end = match task.end_time() {
            Some(t) => t.to_string_hhmm(),
            None => String::from(""),
        };
        let duration = task.duration_hhmm();
        let name = task.name();

        table.add_row(row![date, r -> n, start, end, r -> duration, name]);
    }
    table.printstd();

    Ok(())
}

// Print task summary
fn print_summary(task_summary: TaskSummary, breaks: Vec<Task>) -> Result<()> {
    let container_format = *format::consts::FORMAT_CLEAN;
    let table_format = format::FormatBuilder::new()
        .separator(
            format::LinePosition::Bottom,
            format::LineSeparator::new('-', '-', '-', '-'),
        )
        .separator(
            format::LinePosition::Title,
            format::LineSeparator::new('-', '-', '-', '-'),
        )
        .padding(1, 1)
        .build();

    // daily total
    let mut table_total =
        build_summary_table_structure(row!["Start", "End", "Duration"], table_format);
    table_total.add_row(row![
        l -> task_summary.start().to_string_hhmm(),
        l -> task_summary.end().to_string_hhmm(),
        r -> task_summary.duration_total().to_string_hhmm()
    ]);

    // task Total
    let mut table_durations = build_summary_table_structure(row!["Task", "Duration"], table_format);

    let duration_map = task_summary.duration_by_taskname();
    let mut task_names = duration_map.keys().cloned().collect::<Vec<String>>();
    task_names.sort();

    for task_name in task_names {
        let dur = duration_map.get(&task_name).unwrap().to_string_hhmm();
        table_durations.add_row(row![l -> task_name, r -> dur]);
    }

    // break times
    let mut table_breaks = build_summary_table_structure(row!["Break"], table_format);
    if !breaks.is_empty() {
        for b in breaks {
            let start = b.start_time().to_string_hhmm();
            let end = match b.end_time() {
                Some(t) => t.to_string_hhmm(),
                None => String::from(""),
            };
            table_breaks.add_row(row![start + " - " + &end]);
        }
    } else {
        table_breaks.add_row(row!["NA"]);
    }

    // print summary
    let mut summary_table =
        table!([b => "Summary"], [table_total], [""], [table_durations], [""], [table_breaks]);
    summary_table.set_format(container_format);
    summary_table.printstd();

    Ok(())
}

/// Create summary table template
fn build_summary_table_structure(title: Row, format: format::TableFormat) -> Table {
    let mut tab = Table::new();
    tab.set_format(format);
    tab.set_titles(title);
    tab
}
