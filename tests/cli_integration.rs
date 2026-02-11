//! Integration tests that run the CLI binary.

#[test]
fn cli_help_succeeds_and_outputs_usage() {
    // CARGO_BIN_EXE_<name> uses the binary target name; hyphens require concat! for env!()
    let bin = env!(concat!("CARGO_BIN_EXE_my", "-", "open", "-", "claude"));
    let output = std::process::Command::new(bin)
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
        stdout.contains("my-open-claude") || stdout.contains("prompt"),
        "expected usage text in output"
    );
}
