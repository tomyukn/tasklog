use crate::db::Database;
use anyhow::Result;

/// Run init subcommand.
pub fn run(db: &mut Database, force_init: bool) -> Result<()> {
    initialize_db(db, force_init)?;

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
