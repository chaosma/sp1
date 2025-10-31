pub mod agent;
pub mod compose;
pub mod constants;
pub mod serialization;
pub mod task;

use crate::task::Task;
use anyhow::Result;

pub fn deserialize_task(task: &str) -> Result<Task> {
    serde_json::from_str(task)
        .map_err(|e| anyhow::anyhow!("Error deserializing task {:?}: {}", task, e))
}