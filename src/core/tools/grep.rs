//! Grep tool — search files by regex pattern.

use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;
use walkdir::WalkDir;

use super::{GREP_DEFAULT_MAX_RESULTS, default_search_path, ignore, str_arg, tool_definition};

#[derive(Debug, Deserialize)]
struct GrepArgs {
    pattern: String,
    #[serde(default = "default_search_path")]
    path: String,
    include: Option<String>,
    #[serde(default)]
    context_lines: usize,
    #[serde(default = "default_grep_max_results")]
    max_results: usize,
}

fn default_grep_max_results() -> usize {
    GREP_DEFAULT_MAX_RESULTS
}

pub struct GrepTool;

impl super::Tool for GrepTool {
    fn name(&self) -> &'static str {
        "Grep"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Search files by regex pattern. Returns matching lines with file paths and line numbers.",
            json!({
                "type": "object",
                "required": ["pattern"],
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search in (default: current directory)"
                    },
                    "include": {
                        "type": "string",
                        "description": "File extension filter, e.g. \"rs\", \"ts\" (without dot)"
                    },
                    "context_lines": {
                        "type": "integer",
                        "description": "Lines of context before and after each match (default: 0)"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matching lines to return (default: 50)"
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

    fn execute(&self, args: &Value) -> Result<String, super::ToolError> {
        let parsed: GrepArgs = serde_json::from_value(args.clone())
            .map_err(|e| std::io::Error::other(format!("Invalid arguments: {}", e)))?;

        let re =
            Regex::new(&parsed.pattern).map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let root = std::path::Path::new(&parsed.path);
        if !root.exists() {
            return Err(format!("Path does not exist: {}", parsed.path).into());
        }

        let mut results: Vec<String> = Vec::new();
        let mut total_matches: usize = 0;

        // If path is a file, search just that file
        if root.is_file() {
            search_file(root, &re, &parsed, &mut results, &mut total_matches);
        } else {
            // Walk directory
            let walker = WalkDir::new(root)
                .into_iter()
                .filter_entry(|e| !ignore::is_ignored(e));

            for entry in walker.flatten() {
                if !entry.file_type().is_file() {
                    continue;
                }

                // Extension filter
                if let Some(ref ext) = parsed.include {
                    let file_ext = entry
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    if !file_ext.eq_ignore_ascii_case(ext) {
                        continue;
                    }
                }

                search_file(entry.path(), &re, &parsed, &mut results, &mut total_matches);

                if results.len() >= parsed.max_results {
                    break;
                }
            }
        }

        if results.is_empty() {
            return Ok("No matches found.".to_string());
        }

        let truncated = results.len() < total_matches;
        let mut output = results.join("\n");
        if truncated {
            output.push_str(&format!(
                "\n... ({} more matches truncated)",
                total_matches - results.len()
            ));
        }

        Ok(output)
    }
}

/// Search a single file for regex matches with optional context lines.
fn search_file(
    path: &std::path::Path,
    re: &Regex,
    args: &GrepArgs,
    results: &mut Vec<String>,
    total_matches: &mut usize,
) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return, // skip binary / unreadable files
    };

    let lines: Vec<&str> = content.lines().collect();
    let path_str = path.display().to_string();

    // Find all matching line indices
    let matching: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| re.is_match(line))
        .map(|(i, _)| i)
        .collect();

    if matching.is_empty() {
        return;
    }

    *total_matches += matching.len();

    for &line_idx in &matching {
        if results.len() >= args.max_results {
            return;
        }

        if args.context_lines == 0 {
            results.push(format!("{}:{}:{}", path_str, line_idx + 1, lines[line_idx]));
        } else {
            let start = line_idx.saturating_sub(args.context_lines);
            let end = (line_idx + args.context_lines + 1).min(lines.len());

            for (idx, line) in lines[start..end].iter().enumerate() {
                let i = start + idx;
                let prefix = if i == line_idx { ":" } else { "-" };
                results.push(format!("{}{}{}{}{}", path_str, prefix, i + 1, prefix, line));
            }
        }
    }
}
