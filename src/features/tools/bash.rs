use serde_json::{json, Value};
use std::process::Command;

pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "Bash",
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
