//! ListDir tool — list directory contents recursively with configurable depth.

use serde::Deserialize;
use serde_json::{Value, json};
use walkdir::WalkDir;

use super::{ignore, str_arg, tool_definition};

#[derive(Debug, Deserialize)]
struct ListDirArgs {
    path: String,
    #[serde(default = "default_max_depth")]
    max_depth: usize,
}

fn default_max_depth() -> usize {
    1
}

pub struct ListDirTool;

impl super::Tool for ListDirTool {
    fn name(&self) -> &'static str {
        "ListDir"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "List files and directories at a given path. Use max_depth to explore deeper.",
            json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list"
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Recursion depth (default: 1 = one level). Set higher to explore subdirectories."
                    }
                }
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "path")
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: ListDirArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let root = std::path::Path::new(&parsed.path);
        if !root.exists() {
            return Err(format!("Path does not exist: {}", parsed.path).into());
        }
        if !root.is_dir() {
            return Err(format!("Not a directory: {}", parsed.path).into());
        }

        let walker = WalkDir::new(root)
            .max_depth(parsed.max_depth)
            .into_iter()
            .filter_entry(|e| !ignore::is_ignored(e));

        let mut dirs: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();

        for entry in walker.flatten() {
            // Skip the root directory itself
            if entry.path() == root {
                continue;
            }

            let rel_path = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path());

            let display = rel_path.display().to_string();

            if entry.file_type().is_dir() {
                dirs.push(format!("{}/", display));
            } else {
                files.push(display);
            }
        }

        dirs.sort();
        files.sort();

        // Directories first, then files
        let mut output = dirs;
        output.append(&mut files);

        if output.is_empty() {
            return Ok("Directory is empty.".to_string());
        }

        Ok(output.join("\n"))
    }
}
