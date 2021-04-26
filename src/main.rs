use anyhow::Result;
use clap::{crate_version, Clap};
use dialoguer::Confirm;
use tasklog::db::{get_db_path_from_env_var_or, Database};
use tasklog::task::{Task, TaskList, TaskTime, TimeDisplay, WorkDate};

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

fn initialize_database(db: &mut Database, force: bool) -> Result<()> {
    if !db.is_prepared()? || force {
        db.initialize()?;
        println!("Database created: {}", db.location());
    } else {
        println!(
            "Database already exists: {}\n\
            Use --force to recreate",
            db.location()
        );
    };

    Ok(())
}

fn main() -> Result<()> {
    let root_opts = Opts::parse();
    let db_path = get_db_path_from_env_var_or("tasklog.db")?;
    let break_time_taskname = "break time";

    match root_opts.subcmd {
        SubCommand::Init(opts) => {
            let mut db = Database::connect_rwc(&db_path)?;
            initialize_database(&mut db, opts.force)?;
        }

        SubCommand::Register(opts) => {
            let mut db = Database::connect_rw(&db_path)?;

            if let Err(e) = db.register_taskname(&opts.task_name) {
                println!("{}: {}", e, &opts.task_name);
            }
        }

        SubCommand::Unregister(opts) => {
            let mut db = Database::connect_rw(&db_path)?;

            if let Err(e) = db.unregister_taskname(&opts.task_name) {
                println!("{}: {}", e, &opts.task_name);
            }
        }

        SubCommand::Tasks => {
            let db = Database::connect_r(&db_path)?;

            let tasks = db.get_tasknames()?;
            for (num, taskname) in tasks {
                println!("{0:>2}. {1}", num, taskname);
            }
        }

        SubCommand::Start(opts) => {
            // build start time
            let start_time = match opts.time {
                Some(t) => TaskTime::parse_from_str_hhmm(&t)?,
                None => TaskTime::now(),
            };

            let mut db = Database::connect_rw(&db_path)?;

            // fill end time of the previous task if it is empty
            if let Some(current_task_id) = db.get_current_task_id()? {
                let mut current_task = db.get_task(current_task_id)?;

                let updated_task = current_task.set_end_time(Some(start_time));
                db.update_task(current_task_id, &updated_task)?;

                println!(
                    "{} ended at {}",
                    &updated_task.name(),
                    &start_time.to_string_hhmm()
                );
            }

            // create a new task
            let new_task_name = if opts.break_time {
                String::from(break_time_taskname)
            } else {
                db.get_taskname(opts.task_number.unwrap())?
            };
            let new_task = Task::start(new_task_name.clone(), start_time);
            db.add_task_entry(&new_task)?;

            println!(
                "{} started at {}",
                new_task_name,
                start_time.to_string_hhmm()
            );
        }

        SubCommand::End(opts) => {
            // build end time
            let end_time = match opts.time {
                Some(t) => TaskTime::parse_from_str_hhmm(&t)?,
                None => TaskTime::now(),
            };

            let db = Database::connect_rw(&db_path)?;

            // fill end time of the current task
            if let Some(current_task_id) = db.get_current_task_id()? {
                let mut current_task = db.get_task(current_task_id)?;

                let updated_task = current_task.set_end_time(Some(end_time));
                db.update_task(current_task_id, &updated_task)?;

                db.reset_manager()?;

                println!(
                    "{} ended at {}",
                    &current_task.name(),
                    &end_time.to_string_hhmm()
                );
            }
        }

        SubCommand::List(opts) => {
            let db = Database::connect_rw(&db_path)?;

            let date = match opts.date {
                Some(date) => WorkDate::parse_from_str(&date)?,
                None => WorkDate::now(),
            };

            // show details
            let tasks_with_num = db.get_tasks(opts.all, Some(date))?;
            println!("List");
            println!("----");
            println!(
                "{:<10}  {:<2}  {:<5} - {:<5}  {:<8}  {:<20}",
                "Date", "No", "Start", "End", "Duration", "Task"
            );

            let mut tasks: Vec<Task> = Vec::new();
            let mut breaks: Vec<Task> = Vec::new();

            for (n, task) in tasks_with_num {
                println!(
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
                );

                if task.name() == break_time_taskname {
                    breaks.push(task);
                } else {
                    tasks.push(task);
                };
            }

            // show summary
            if !opts.all {
                if let Some(summary) = TaskList::new(tasks).summary() {
                    println!("\nSummary");
                    println!("-------");
                    println!("   Start: {}", summary.start().to_string_hhmm());
                    println!("     End: {}", summary.end().to_string_hhmm());
                    println!("Duration: {}\n", summary.duration_total().to_string_hhmm());

                    // total time
                    let mut names: Vec<String> =
                        summary.duration_by_taskname().keys().cloned().collect();
                    names.sort();
                    for name in names {
                        let dur = summary
                            .duration_by_taskname()
                            .get(&name)
                            .unwrap()
                            .to_string_hhmm();
                        println!("{:<5}  {}", dur, name);
                    }

                    // break time
                    println!("\n[Break]");
                    for break_time in breaks {
                        println!(
                            "{} - {}",
                            break_time.start_time().to_string_hhmm(),
                            match break_time.end_time() {
                                Some(t) => t.to_string_hhmm(),
                                None => String::from(""),
                            }
                        );
                    }
                    println!("")
                }
            }
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
            db.delete_task(task_id)?;
            println!("task {} deleted", opts.task_number);
        }

        SubCommand::ShowManager => {
            println!(
                "Caution: This command shows the internal status for debugging the application.\n"
            );
            let db = Database::connect_r(&db_path)?;
            let manager = db.get_manager()?;
            println!("{:?}", manager);
        }

        SubCommand::ResetManager => {
            println!("Caution: This operation can be dangerous. It may break your database.\n");

            let proceed = Confirm::new()
                .with_prompt("Do you wish to continue?")
                .wait_for_newline(false)
                .default(false)
                .show_default(true)
                .interact()?;
            if proceed {
                let db = Database::connect_rw(&db_path)?;
                db.reset_manager()?;
                println!("Manager has been reset.");
            } else {
                println!("Oparation canceled.");
            };
        }
    }

    Ok(())
}
