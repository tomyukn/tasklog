use crate::db::Database;
use anyhow::Result;
use std::path::PathBuf;

/// Run init subcommand.
pub fn run(db_path: PathBuf, force_init: bool) -> Result<()> {
    let mut db = Database::connect_rwc(&db_path)?;
    initialize_db(&mut db, force_init)?;

    Ok(())
}

/// Initialize a database.
fn initialize_db(db: &mut Database, force: bool) -> Result<()> {
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
