use crate::db::Database;
use crate::task::{TaskList, TaskSummary, TimeDisplay, WorkDate};
use anyhow::Result;
use prettytable::{format, table, Row, Table};

/// Print task log
pub fn run(db: &Database, show_all: bool, date: Option<String>) -> Result<()> {
    let date = build_date(date, WorkDate::now())?;
    let tasks = db.get_tasks(show_all, Some(date))?;

    // show list
    print_list(tasks.clone())?;

    // show summary
    if !show_all {
        if let Some(task_summary) = tasks.summary() {
            print!("\n");
            print_summary(task_summary)?;
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
fn print_list(tasklist: TaskList) -> Result<()> {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_CLEAN);

    // title
    table.set_titles(row![b => "Date", "No", "Start", "End", "Duration", "Task"]);

    // contents
    for (n, task) in tasklist {
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
fn print_summary(task_summary: TaskSummary) -> Result<()> {
    // table formats
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

    // build tables
    let table_daily_overall = build_daily_total_table(&task_summary, table_format);
    let table_task_durations = build_task_total_table(&task_summary, table_format);
    let table_break_times = build_break_time_table(&task_summary, table_format);

    // print
    let mut summary_table = table!(
        [b => "Summary"],
        [table_daily_overall], [""],
        [table_task_durations], [""],
        [table_break_times]
    );
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

/// Create daily total table which contains overall start time, end time, and duration.
fn build_daily_total_table(task_summary: &TaskSummary, format: format::TableFormat) -> Table {
    let start = task_summary.start_time().to_string_hhmm();
    let end = task_summary.end_time().to_string_hhmm();
    let duration = task_summary.duration_total().to_string_hhmm();

    let mut tab = build_summary_table_structure(row!["Start", "End", "Duration"], format);
    tab.add_row(row![l -> start, l -> end, r -> duration]);

    tab
}

/// Create task duration table.
fn build_task_total_table(task_summary: &TaskSummary, format: format::TableFormat) -> Table {
    let duration_map = task_summary.duration_by_taskname(); // key: task name, value: duration

    let mut names = duration_map.keys().cloned().collect::<Vec<String>>();
    names.sort();

    let mut tab = build_summary_table_structure(row!["Task", "Duration"], format);
    for task_name in names {
        let dur = duration_map.get(&task_name).unwrap().to_string_hhmm();
        tab.add_row(row![l -> task_name, r -> dur]);
    }

    tab
}

/// Create break time list table.
fn build_break_time_table(task_summary: &TaskSummary, format: format::TableFormat) -> Table {
    let mut tab = build_summary_table_structure(row!["Break"], format);

    if task_summary.break_times().is_empty() {
        tab.add_row(row!["NA"]);
    } else {
        for break_time in task_summary.break_times() {
            let start = break_time.start_time().to_string_hhmm();
            let end = match break_time.end_time() {
                Some(t) => t.to_string_hhmm(),
                None => String::from(""),
            };
            tab.add_row(row![start + " - " + &end]);
        }
    }

    tab
}
