use crate::db::Database;
use anyhow::Result;
use dialoguer::Confirm;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Print the manager database entry.
pub fn show(db: &Database) -> Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.lock();

    write_boldred(&mut stderr, "Warning")?;
    writeln!(
        &mut stderr,
        ": This command shows the internal status for debugging the application.\n"
    )?;

    let manager = db.get_manager()?;
    println!("{:?}", manager);

    Ok(())
}

/// Reset the manager status.
pub fn reset(db: &Database) -> Result<()> {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    stderr.lock();

    write_boldred(&mut stderr, "Warning")?;
    writeln!(
        &mut stderr,
        ": This operation can be dangerous. It may break your database.\n",
    )?;

    let proceed = Confirm::new()
        .with_prompt("Do you wish to continue?")
        .wait_for_newline(false)
        .default(false)
        .show_default(true)
        .interact()?;
    if proceed {
        db.reset_manager()?;
        println!("\nManager has been reset.");
    } else {
        println!("\nOparation canceled.");
    };

    Ok(())
}

/// Bold red font
fn write_boldred(out: &mut StandardStream, s: &str) -> std::io::Result<()> {
    out.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    write!(out, "{}", s)?;
    out.reset()?;
    Ok(())
}
