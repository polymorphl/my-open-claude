pub mod read;

use serde_json::Value;

pub fn definitions() -> Vec<Value> {
    vec![read::definition()]
}
