//! Build script: validates builtin-commands.json at compile time.

use std::path::PathBuf;

fn main() {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by Cargo");
    let config_path: PathBuf = [&manifest_dir, "config", "builtin-commands.json"]
        .iter()
        .collect();
    let json = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read {}: {}. builtin-commands.json must exist and be valid.",
            config_path.display(),
            e
        )
    });
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct BuiltinCommandEntry {
        name: String,
        description: String,
        prompt_prefix: String,
        mode: String,
    }
    let _: Vec<BuiltinCommandEntry> = serde_json::from_str(&json).unwrap_or_else(|e| {
        panic!(
            "builtin-commands.json is invalid JSON: {}. Fix the file and rebuild.",
            e
        )
    });
}
