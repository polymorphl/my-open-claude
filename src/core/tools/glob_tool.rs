//! Glob tool — find files matching a glob pattern.

use globset::Glob;
use serde::Deserialize;
use serde_json::{Value, json};
use walkdir::WalkDir;

use super::{ignore, str_arg, tool_definition, default_search_path, GLOB_DEFAULT_MAX_RESULTS};

#[derive(Debug, Deserialize)]
struct GlobArgs {
    pattern: String,
    #[serde(default = "default_search_path")]
    path: String,
    #[serde(default = "default_glob_max_results")]
    max_results: usize,
}

fn default_glob_max_results() -> usize {
    GLOB_DEFAULT_MAX_RESULTS
}

pub struct GlobTool;

impl super::Tool for GlobTool {
    fn name(&self) -> &'static str {
        "Glob"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Find files matching a glob pattern (e.g. **/*.rs, src/**/*.test.ts).",
            json!({
                "type": "object",
                "required": ["pattern"],
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match files (e.g. \"**/*.rs\", \"src/**/*.ts\")"
                    },
                    "path": {
                        "type": "string",
                        "description": "Root directory to search from (default: current directory)"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of file paths to return (default: 100)"
                    }
                }
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        let pattern = str_arg(args, "pattern");
        let path = str_arg(args, "path");
        if path.is_empty() || path == "." {
            pattern
        } else {
            format!("{} in {}", pattern, path)
        }
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: GlobArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let matcher = Glob::new(&parsed.pattern)
            .map_err(|e| format!("Invalid glob pattern: {}", e))?
            .compile_matcher();

        let root = std::path::Path::new(&parsed.path);
        if !root.exists() {
            return Err(format!("Path does not exist: {}", parsed.path).into());
        }

        let walker = WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| !ignore::is_ignored(e));

        let mut results: Vec<String> = Vec::new();
        let mut total: usize = 0;

        for entry in walker.flatten() {
            if !entry.file_type().is_file() {
                continue;
            }

            let rel_path = entry.path().strip_prefix(root).unwrap_or(entry.path());

            if matcher.is_match(rel_path) {
                total += 1;
                if results.len() < parsed.max_results {
                    results.push(rel_path.display().to_string());
                }
            }
        }

        if results.is_empty() {
            return Ok("No files matched the pattern.".to_string());
        }

        let mut output = results.join("\n");
        if total > parsed.max_results {
            output.push_str(&format!(
                "\n... ({} more files truncated)",
                total - parsed.max_results
            ));
        }

        Ok(output)
    }
}
