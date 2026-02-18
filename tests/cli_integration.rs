//! Integration tests that run the CLI binary.

/// App name from Cargo.toml (matches core::app::NAME used by the binary).
const APP_NAME: &str = env!("CARGO_PKG_NAME");

fn bin() -> std::process::Command {
    // CARGO_BIN_EXE_<name> is set by Cargo for each binary target.
    let bin = env!(concat!("CARGO_BIN_EXE_", env!("CARGO_PKG_NAME")));
    let mut cmd = std::process::Command::new(bin);
    cmd.env_remove("OPENROUTER_API_KEY");
    cmd
}

#[test]
fn cli_help_succeeds_and_outputs_usage() {
    let output = bin()
        .arg("--help")
        .output()
        .expect("binary not found - run cargo build first");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.is_empty());
    assert!(
        stdout.contains(APP_NAME) || stdout.contains("prompt"),
        "expected usage text in output"
    );
}

#[test]
fn cli_version_succeeds() {
    let output = bin()
        .arg("--version")
        .output()
        .expect("binary not found - run cargo build first");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(APP_NAME));
}

#[test]
fn cli_prompt_without_api_key_exits_with_error() {
    // Run from temp dir so dotenv() won't load .env from project root
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let output = bin()
        .arg("-p")
        .arg("hello")
        .current_dir(tmp.path())
        .output()
        .expect("binary not found - run cargo build first");

    assert!(
        !output.status.success(),
        "expected failure when OPENROUTER_API_KEY is not set"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("OPENROUTER_API_KEY"),
        "expected API key error message, got: {}",
        stderr
    );
}

#[test]
fn cli_config_outputs_paths_and_status() {
    let tmp = tempfile::TempDir::new().expect("temp dir");
    let output = bin()
        .arg("config")
        .current_dir(tmp.path())
        .output()
        .expect("binary not found - run cargo build first");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Config:"), "expected Config: in output");
    assert!(stdout.contains("API key:"), "expected API key: in output");
}

#[test]
fn cli_completions_bash_produces_script() {
    let output = bin()
        .arg("completions")
        .arg("bash")
        .output()
        .expect("binary not found - run cargo build first");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("_my-open-claude") || stdout.contains(APP_NAME),
        "expected completion script for {}",
        APP_NAME
    );
}
