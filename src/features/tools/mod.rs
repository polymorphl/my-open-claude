pub mod read;
pub mod write;

use serde_json::Value;

pub fn definitions() -> Vec<Value> {
    vec![read::definition(), write::definition()]
}
