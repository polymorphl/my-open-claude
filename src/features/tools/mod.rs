mod read;

pub use read::{execute_tool_call, ResponseOutput};

use serde_json::Value;

pub fn definitions() -> Vec<Value> {
    vec![read::definition()]
}
