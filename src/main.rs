use anyhow::Result;
use clap::{crate_version, Clap};
use dialoguer::Confirm;
use std::io::Write;
use tasklog::db::{get_db_path_from_env_var_or, Database};
use tasklog::subcommand;
use tasklog::task::{TaskTime, TimeDisplay, WorkDate};
use termcolor::{ColorChoice, ColorSpec, StandardStream, WriteColor};

// command line arguments
#[derive(Clap)]
#[clap(
    name = "tasklog",
    about = "Logging tasks",
    version = crate_version!()
)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

// subcommands
#[derive(Clap)]
enum SubCommand {
    #[clap(
        about = "Initializes a database or reinitialize an existing one",
        version = crate_version!()
    )]
    Init(InitOpts),

    #[clap(
        about = "Registars a task name",
        version = crate_version!()
    )]
    Register(RegisterOpts),

    #[clap(
        about = "Unregistars a task name",
        version = crate_version!()
    )]
    Unregister(UnregisterOpts),

    #[clap(
        about = "Shows registered task names",
        version = crate_version!()
    )]
    Tasks,

    #[clap(
        about = "Starts a task",
        version = crate_version!()
    )]
    Start(StartOpts),

    #[clap(
        about = "Ends a task",
        version = crate_version!()
    )]
    End(EndOpts),

    #[clap(
        about = "Lists logged task entries",
        version = crate_version!()
    )]
    List(ListOpts),

    #[clap(
        about = "Updates a task entry",
        version = crate_version!()
    )]
    Update(UpdateOpts),

    #[clap(
        about = "Deletes a task entry",
        version = crate_version!()
    )]
    Delete(DeleteOpts),

    #[clap(
        about = "Shows the internal status for debugging",
        version = crate_version!()
    )]
    ShowManager,

    #[clap(
        about = "Resets the internal status for debugging",
        version = crate_version!()
    )]
    ResetManager,
}

#[derive(Clap)]
struct InitOpts {
    #[clap(
        short,
        long,
        about = "Force initializes the database if it already exists"
    )]
    force: bool,
}

#[derive(Clap)]
struct RegisterOpts {
    task_name: String,
}

#[derive(Clap)]
struct UnregisterOpts {
    task_name: String,
}

#[derive(Clap)]
struct StartOpts {
    #[clap(about = "Task number in the task name list")]
    task_number: Option<u32>,
    #[clap(
        short,
        long,
        about = "Starts a break time",
        conflicts_with = "task-number"
    )]
    break_time: bool,
    #[clap(short, long, about = "Start time, `HHMM` format")]
    time: Option<String>,
}

#[derive(Clap)]
struct EndOpts {
    #[clap(short, long, about = "End time, `HHMM` format")]
    time: Option<String>,
}

#[derive(Clap)]
struct ListOpts {
    #[clap(short, long, about = "Shows all task logs instead of today's")]
    all: bool,
    #[clap(short, long, about = "Date shown")]
    date: Option<String>,
}

#[derive(Clap)]
struct UpdateOpts {
    #[clap(about = "Task number in the task list")]
    task_number: u32,
    #[clap(possible_values = &["name", "start", "end"], about = "Update target")]
    target: String,
    #[clap(about = "New value")]
    value: String,
}

#[derive(Clap)]
struct DeleteOpts {
    #[clap(about = "Task number in the task list")]
    task_number: u32,
}

fn write_bold(out: &mut StandardStream, s: &str) -> std::io::Result<()> {
    out.set_color(ColorSpec::new().set_bold(true))?;
    write!(out, "{}", s)?;
    out.reset()?;
    Ok(())
}

fn main() -> Result<()> {
    let root_opts = Opts::parse();
    let db_path = get_db_path_from_env_var_or("tasklog.db")?;
    let break_time_taskname = "break time";

    match root_opts.subcmd {
        SubCommand::Init(opts) => {
            subcommand::init::run(db_path, opts.force)?;
        }

        SubCommand::Register(opts) => {
            subcommand::register::run(db_path, &opts.task_name)?;
        }

        SubCommand::Unregister(opts) => {
            subcommand::unregister::run(db_path, &opts.task_name)?;
        }

        SubCommand::Tasks => {
            subcommand::show_tasks::run(db_path)?;
        }

        SubCommand::Start(opts) => {
            subcommand::start::run(
                db_path,
                opts.task_number,
                opts.break_time,
                opts.time,
                break_time_taskname,
            )?;
        }

        SubCommand::End(opts) => {
            subcommand::end::run(db_path, opts.time)?;
        }

        SubCommand::List(opts) => {
            subcommand::list_log::run(db_path, opts.all, opts.date, break_time_taskname)?;
        }

        SubCommand::Update(opts) => {
            let db = Database::connect_rw(&db_path)?;
            let working_date = WorkDate::now();

            let task_id = db.get_task_id_by_seqnum(opts.task_number, working_date)?;
            let mut task = db.get_task(task_id)?;

            if opts.target == "name" {
                task.set_name(opts.value);
            } else if opts.target == "start" {
                task.set_start_time(TaskTime::parse_from_str_hhmm(&opts.value)?);
            } else if opts.target == "end" {
                task.set_end_time(Some(TaskTime::parse_from_str_hhmm(&opts.value)?));
            }

            db.update_task(task.id().unwrap(), &task)?;
        }

        SubCommand::Delete(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            let working_date = WorkDate::now();

            let task_id = db.get_task_id_by_seqnum(opts.task_number, working_date)?;
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
                println!("\ntask {} deleted", opts.task_number);
            } else {
                println!("\nOparation canceled.");
            };
        }

        SubCommand::ShowManager => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.lock();

            write_bold(
                &mut stderr,
                "Warning: This command shows the internal status for debugging the application.\n\n",
            )?;

            let db = Database::connect_r(&db_path)?;
            let manager = db.get_manager()?;
            println!("{:?}", manager);
        }

        SubCommand::ResetManager => {
            let mut stderr = StandardStream::stderr(ColorChoice::Auto);
            stderr.lock();

            write_bold(
                &mut stderr,
                "Warning: This operation can be dangerous. It may break your database.\n",
            )?;

            let proceed = Confirm::new()
                .with_prompt("Do you wish to continue?")
                .wait_for_newline(false)
                .default(false)
                .show_default(true)
                .interact()?;
            if proceed {
                let db = Database::connect_rw(&db_path)?;
                db.reset_manager()?;
                println!("\nManager has been reset.");
            } else {
                println!("\nOparation canceled.");
            };
        }
    }

    Ok(())
}
