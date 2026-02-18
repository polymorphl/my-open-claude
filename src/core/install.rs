//! Install the binary to ~/.cargo/bin from the current project.
//!
//! Runs `cargo install --path .` when invoked from a directory containing Cargo.toml.

use std::env;
use std::env::consts::EXE_SUFFIX;

/// Install the binary to the user's cargo bin directory.
///
/// Requires Cargo.toml in the current directory. Spawns `cargo install --path .`.
///
/// # Errors
/// Returns an error if the current directory cannot be determined, Cargo.toml is missing,
/// or `cargo install` fails. Exits the process on failure with an appropriate message.
pub fn run_install() -> Result<(), Box<dyn std::error::Error>> {
    let cwd = env::current_dir()?;
    let cargo_toml = cwd.join("Cargo.toml");
    if !cargo_toml.exists() {
        eprintln!(
            "Error: Cargo.toml not found in current directory.\n\
             Run from the project directory containing Cargo.toml, or use 'cargo install --path .'"
        );
        std::process::exit(1);
    }
    let status = std::process::Command::new("cargo")
        .args(["install", "--path", "."])
        .current_dir(&cwd)
        .status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    let cargo_home = env::var("CARGO_HOME").unwrap_or_else(|_| {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_default();
        format!("{}/.cargo", home)
    });
    let install_path = format!("{}/bin/my-open-claude{}", cargo_home, EXE_SUFFIX);
    println!("Installed to {}", install_path);
    Ok(())
}
