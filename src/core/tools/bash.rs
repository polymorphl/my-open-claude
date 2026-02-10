use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;

use super::{str_arg, tool_definition};

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

#[derive(Debug, Deserialize)]
pub struct BashArgs {
    pub command: String,
}

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

pub struct BashTool;

impl super::Tool for BashTool {
    fn name(&self) -> &'static str {
        "Bash"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Execute a shell command",
            json!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command to execute"
                    }
                }
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "command")
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: BashArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", &parsed.command])
                .output()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(&parsed.command)
                .output()
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                if !stderr.is_empty() && !stdout.is_empty() {
                    Ok(format!("{}\n{}", stderr, stdout))
                } else if !stderr.is_empty() {
                    Ok(stderr.to_string())
                } else {
                    Ok(stdout.to_string())
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}
