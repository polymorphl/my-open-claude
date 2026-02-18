//! Self-update from GitHub releases.
//!
//! Uses the `self_update` crate to download the latest release and replace
//! the current binary in place.

use self_update::Status;

use crate::core::app;

fn format_update_error(err: &(dyn std::error::Error + 'static)) -> String {
    let msg = err.to_string().to_lowercase();
    if msg.contains("network")
        || msg.contains("connection")
        || msg.contains("timed out")
        || msg.contains("dns")
    {
        "Could not fetch releases. Check your network connection.".to_string()
    } else if msg.contains("not found") || msg.contains("404") {
        "No release found. The project may not have published releases yet.".to_string()
    } else if msg.contains("no asset") || msg.contains("target") {
        format!(
            "No pre-built binary for your platform. Build from source: \
             https://github.com/{}/{}/releases",
            app::VENDOR,
            app::NAME
        )
    } else {
        format!("Update failed: {}", err)
    }
}

/// Check if an update is available without downloading.
///
/// Fetches release metadata from GitHub and compares with the current version.
///
/// # Errors
/// Returns an error if the release check fails (network, API, etc.).
pub fn run_update_check() -> Result<(), Box<dyn std::error::Error>> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner(app::VENDOR)
        .repo_name(app::NAME)
        .bin_name(app::NAME)
        .current_version(app::VERSION)
        .no_confirm(true)
        .show_download_progress(true)
        .build()?;
    let current = updater.current_version();
    let release = updater.get_latest_release().map_err(|e| {
        let msg = format_update_error(&e);
        std::io::Error::other(msg)
    })?;
    let latest = release.version;
    if semver::Version::parse(&latest)? > semver::Version::parse(&current)? {
        println!("Update available: v{} (current: v{})", latest, current);
    } else {
        println!("Already up to date (v{})", current);
    }
    Ok(())
}

/// Run the self-update: fetch latest release from GitHub and replace the binary.
///
/// # Errors
/// Returns an error if the update check, download, or replacement fails.
pub fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    let updater = self_update::backends::github::Update::configure()
        .repo_owner(app::VENDOR)
        .repo_name(app::NAME)
        .bin_name(app::NAME)
        .current_version(app::VERSION)
        .no_confirm(true)
        .show_download_progress(true)
        .build()?;
    let status = updater.update().map_err(|e| {
        let msg = format_update_error(&e);
        std::io::Error::other(msg)
    })?;

    match status {
        Status::UpToDate(v) => println!("Already up to date (v{})", v),
        Status::Updated(v) => println!("Updated to v{}!", v),
    }
    Ok(())
}
