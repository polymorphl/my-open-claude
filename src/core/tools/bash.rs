use serde_json::{json, Value};
use std::process::Command;

/// Tool name as sent to the API and used for dispatch.
pub const NAME: &str = "Bash";

/// Command prefixes (normalized, lowercase) that are considered destructive and require confirmation.
const DESTRUCTIVE_PREFIXES: &[&str] = &[
    "rm ",
    "rm -",
    "rmdir ",
    "del ",   // Windows
    "rd ",    // Windows (remove directory)
    "mv ",    // can overwrite or remove
    "unlink ",
];

fn normalized_command(cmd: &str) -> String {
    cmd.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns true if the command is considered destructive (e.g. rm, rmdir) and should require user confirmation.
pub fn is_destructive(command: &str) -> bool {
    let n = normalized_command(command);
    if n.is_empty() {
        return false;
    }
    DESTRUCTIVE_PREFIXES
        .iter()
        .any(|&prefix| n.starts_with(prefix))
}

pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": NAME,
            "description": "Execute a shell command",
            "parameters": {
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute"
                    }
                }
            }
        }
    })
}

pub fn execute(command: &str) -> String {
    // Execute the command using the shell
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
    };

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Combine stdout and stderr, with stderr first if both exist
            if !stderr.is_empty() && !stdout.is_empty() {
                format!("{}\n{}", stderr, stdout)
            } else if !stderr.is_empty() {
                stderr.to_string()
            } else {
                stdout.to_string()
            }
        }
        Err(e) => format!("Error executing command: {}", e),
    }
}
