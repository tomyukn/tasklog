use crate::db::Database;
use crate::task::{Task, TaskList, TimeDisplay, WorkDate};
use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

pub fn run(
    db_path: PathBuf,
    show_all: bool,
    date: Option<String>,
    break_taskname: &str,
) -> Result<()> {
    let db = Database::connect_rw(&db_path)?;

    let date = match date {
        Some(s) => WorkDate::parse_from_str(&s)?,
        None => WorkDate::now(),
    };
    let tasks_with_num = db.get_tasks(show_all, Some(date))?;

    // show details
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    stdout.lock();

    write_bold(&mut stdout, "List\n")?;
    writeln!(
        &mut stdout,
        "{:<10}  {:<2}  {:<5} - {:<5}  {:<8}  {:<20}",
        "Date", "No", "Start", "End", "Duration", "Task"
    )?;

    let mut tasks: Vec<Task> = Vec::new();
    let mut breaks: Vec<Task> = Vec::new();

    for (n, task) in tasks_with_num {
        writeln!(
            &mut stdout,
            "{:<10}  {:>2}  {:<5} - {:<5}  {:<8}  {:<20}",
            task.working_date().to_string(),
            n,
            task.start_time().to_string_hhmm(),
            match task.end_time() {
                Some(t) => t.to_string_hhmm(),
                None => String::from(""),
            },
            task.duration_hhmm(),
            task.name(),
        )?;

        if task.name() == break_taskname {
            breaks.push(task);
        } else {
            tasks.push(task);
        };
    }
    writeln!(&mut stdout, "")?;

    // show summary
    if !show_all {
        if let Some(summary) = TaskList::new(tasks).summary() {
            write_bold(&mut stdout, "Start    : ")?;
            writeln!(&mut stdout, "{}", summary.start().to_string_hhmm())?;

            write_bold(&mut stdout, "End      : ")?;
            writeln!(&mut stdout, "{}", summary.end().to_string_hhmm())?;

            write_bold(&mut stdout, "Duration : ")?;
            writeln!(
                &mut stdout,
                "{}\n",
                summary.duration_total().to_string_hhmm()
            )?;

            // total time
            let mut names: Vec<String> = summary.duration_by_taskname().keys().cloned().collect();
            names.sort();

            write_bold(&mut stdout, "Task duration\n")?;
            for name in names {
                let dur = summary
                    .duration_by_taskname()
                    .get(&name)
                    .unwrap()
                    .to_string_hhmm();
                println!("{:<5}  {}", dur, name);
            }

            // break time
            write_bold(&mut stdout, "\nBreak\n")?;
            for break_time in breaks {
                writeln!(
                    &mut stdout,
                    "{} - {}",
                    break_time.start_time().to_string_hhmm(),
                    match break_time.end_time() {
                        Some(t) => t.to_string_hhmm(),
                        None => String::from(""),
                    }
                )?;
            }
            println!("")
        }
    }

    Ok(())
}

fn write_bold(out: &mut StandardStream, s: &str) -> std::io::Result<()> {
    out.set_color(ColorSpec::new().set_bold(true))?;
    write!(out, "{}", s)?;
    out.reset()?;
    Ok(())
}
