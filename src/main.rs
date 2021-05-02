use anyhow::Result;
use clap::{crate_version, Clap};
use tasklog::db::{get_db_path_from_env_var_or, Database};
use tasklog::subcommand;

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

fn main() -> Result<()> {
    const BREAK_TIME_TASKNAME: &str = "break time";

    let root_opts = Opts::parse();
    let db_path = get_db_path_from_env_var_or("tasklog.db")?;

    match root_opts.subcmd {
        SubCommand::Init(opts) => {
            let mut db = Database::connect_rwc(&db_path)?;
            subcommand::init::run(&mut db, opts.force)?;
        }

        SubCommand::Register(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            subcommand::register::run(&mut db, &opts.task_name)?;
        }

        SubCommand::Unregister(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            subcommand::unregister::run(&mut db, &opts.task_name)?;
        }

        SubCommand::Tasks => {
            let db = Database::connect_r(&db_path)?;
            subcommand::show_tasks::run(&db)?;
        }

        SubCommand::Start(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            subcommand::start::run(
                &mut db,
                opts.task_number,
                opts.break_time,
                opts.time,
                BREAK_TIME_TASKNAME,
            )?;
        }

        SubCommand::End(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            subcommand::end::run(&mut db, opts.time)?;
        }

        SubCommand::List(opts) => {
            let db = Database::connect_rw(&db_path)?;
            subcommand::list_log::run(&db, opts.all, opts.date)?;
        }

        SubCommand::Update(opts) => {
            let db = Database::connect_rw(&db_path)?;
            subcommand::update::run(&db, opts.task_number, opts.target, opts.value)?;
        }

        SubCommand::Delete(opts) => {
            let mut db = Database::connect_rw(&db_path)?;
            subcommand::delete::run(&mut db, opts.task_number)?;
        }

        SubCommand::ShowManager => {
            let db = Database::connect_r(&db_path)?;
            subcommand::manager::show(&db)?;
        }

        SubCommand::ResetManager => {
            let db = Database::connect_rw(&db_path)?;
            subcommand::manager::reset(&db)?;
        }
    }

    Ok(())
}
