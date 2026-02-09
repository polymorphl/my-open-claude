pub mod bash;
pub mod read;
pub mod write;

use serde_json::Value;

pub fn definitions() -> Vec<Value> {
    vec![bash::definition(), read::definition(), write::definition()]
}
