use crate::constants::{LOCAL_HUGE_TLB_PATH, LOCAL_RAMDISK_PATH, SHARED_RAMDISK_PATH};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fmt::Debug;
use std::path::PathBuf;

/// Common trait for all tasks that handle input blob data
///
/// This trait provides methods for managing input blob data path for different types of tasks.
pub trait WithInputBlob {
    /// Gets the paths where input blob data should be stored/retrieved
    fn get_input_paths(&self) -> Vec<PathBuf>;
}

/// - Producer: External service like Reth node
/// - Consumer: Core Shard Checkpointer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoreShardCheckpointTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// ID of the block being checkpointed
    pub block_id: String,
    /// ID of the shard being checkpointed
    pub shard_id: u32,
    /// Path to the program being executed
    pub program_path: String,
    /// Path to the cached program
    pub cached_program_path: String,
}

impl WithInputBlob for CoreShardCheckpointTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(format!(
            "{}/{}/checkpoint/{}",
            SHARED_RAMDISK_PATH, self.root_id, self.block_id
        ))]
    }
}
/// - Producer: External service like Reth node
/// - Consumer: Special Checkpointer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpecialCheckpointTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// ID of the block being checkpointed
    pub block_id: String,
    /// Path to the program being executed
    pub program_path: String,
    /// Path to the cached program
    pub cached_program_path: String,
}

impl WithInputBlob for SpecialCheckpointTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(format!(
            "{}/{}/checkpoint/{}",
            SHARED_RAMDISK_PATH, self.root_id, self.block_id
        ))]
    }
}

impl SpecialCheckpointTask {
    pub fn get_output_prefix(&self) -> PathBuf {
        PathBuf::from(format!("{}/{}/shard/", LOCAL_RAMDISK_PATH, self.root_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShardTaskType {
    Core,
    Precompile,
    GlobalMemory,
}

/// - Producer: Master Checkpointer
/// - Consumer: Shard Prover
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShardTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// ID of the current shard being processed
    pub shard_id: u32,
    /// Path to the program being executed
    pub program_path: String,
    /// Type of the shard task
    pub task_type: ShardTaskType,
}

impl WithInputBlob for ShardTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        // Different types of shard tasks have different input paths.
        // For core shard tasks, the input path is the shard itself.
        // For global memory tasks, it takes 4 input files. The input paths are the {shard itself}-0, {shard itself}-1,
        // the shard ends, and the instruction count.
        // For precompile tasks, it takes 3 input files. The input paths are the shard itself, the shard ends, and
        // the instruction count.
        let local_ramdisk_dir = format!("{}/{}/shard", LOCAL_RAMDISK_PATH, self.root_id);
        let local_huge_tlb_dir = format!("{}/{}/shard", LOCAL_HUGE_TLB_PATH, self.root_id);
        match self.task_type {
            ShardTaskType::Core => {
                vec![PathBuf::from(format!("{}/{}", local_huge_tlb_dir, self.shard_id))]
            }
            ShardTaskType::GlobalMemory => vec![
                PathBuf::from(format!("{}/{}-0", local_huge_tlb_dir, self.shard_id)),
                PathBuf::from(format!("{}/{}-1", local_huge_tlb_dir, self.shard_id)),
                PathBuf::from(format!("{}/shard_ends", local_ramdisk_dir)),
                PathBuf::from(format!("{}/instr_count", local_ramdisk_dir)),
            ],
            ShardTaskType::Precompile => vec![
                PathBuf::from(format!("{}/{}", local_huge_tlb_dir, self.shard_id)),
                PathBuf::from(format!("{}/shard_ends", local_ramdisk_dir)),
                PathBuf::from(format!("{}/instr_count", local_ramdisk_dir)),
            ],
        }
    }
}

/// - Producer: Recursion GPU Prover
/// - Consumer: Recursion Composer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComposeTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// Height in the recursion tree
    pub height: u32,
    /// Index within the current height level
    pub index: u32,
}

impl WithInputBlob for ComposeTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(format!(
            "{}/{}/compose/{}-{}",
            SHARED_RAMDISK_PATH, self.root_id, self.height, self.index
        ))]
    }
}

/// - Producer: Recursion Composer
/// - Consumer: Recursion CPU Prover
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecursionCpuTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// Height in the recursion tree
    pub height: u32,
    /// Index within the current height level
    pub index: u32,
    /// Total number of shards
    pub total_shards: Option<u32>,
}

impl WithInputBlob for RecursionCpuTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        if self.height == 0 {
            // 1-1 recursion
            vec![
                PathBuf::from(format!(
                    "{}/{}/recursion_cpu/{}-{}",
                    SHARED_RAMDISK_PATH, self.root_id, self.height, self.index
                )),
                PathBuf::from(format!("{}/program/eth-block-vk.bin", SHARED_RAMDISK_PATH)),
            ]
        } else {
            // 2-1 recursion
            (0..2)
                .map(|i| {
                    PathBuf::from(format!(
                        "{}/{}/recursion_cpu/{}-{}-{}",
                        SHARED_RAMDISK_PATH, self.root_id, self.height, self.index, i
                    ))
                })
                .collect()
        }
    }
}

/// - Producer: Recursion CPU Prover
/// - Consumer: Recursion GPU Prover
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecursionGpuTask {
    /// Unique identifier to track and organize related tasks
    pub root_id: String,
    /// Height in the recursion tree
    pub height: u32,
    /// Index within the current height level
    pub index: u32,
    /// Total number of shards
    pub total_shards: Option<u32>,
}

impl WithInputBlob for RecursionGpuTask {
    fn get_input_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from(format!(
            "{}/{}/recursion_gpu/{}-{}",
            LOCAL_RAMDISK_PATH, self.root_id, self.height, self.index
        ))]
    }
}

/// Enum representing all possible task types in the proving pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Task {
    CoreShardCheckpoint(CoreShardCheckpointTask),
    SpecialCheckpoint(SpecialCheckpointTask),
    Shard(ShardTask),
    Compose(ComposeTask),
    RecursionCpu(RecursionCpuTask),
    RecursionGpu(RecursionGpuTask),
}

impl Task {
    /// Gets the data path prefix for the task's data
    pub fn get_input_paths(&self) -> Vec<PathBuf> {
        match self {
            Task::CoreShardCheckpoint(task) => task.get_input_paths(),
            Task::SpecialCheckpoint(task) => task.get_input_paths(),
            Task::Shard(task) => task.get_input_paths(),
            Task::Compose(task) => task.get_input_paths(),
            Task::RecursionCpu(task) => task.get_input_paths(),
            Task::RecursionGpu(task) => task.get_input_paths(),
        }
    }

    /// Returns a reference to the core shard checkpoint task if this is a CoreShardCheckpoint variant
    pub fn as_core_shard_checkpoint(&self) -> Option<&CoreShardCheckpointTask> {
        match self {
            Task::CoreShardCheckpoint(task) => Some(task),
            _ => None,
        }
    }

    /// Returns a reference to the special checkpoint task if this is a SpecialCheckpoint variant
    pub fn as_special_checkpoint(&self) -> Option<&SpecialCheckpointTask> {
        match self {
            Task::SpecialCheckpoint(task) => Some(task),
            _ => None,
        }
    }

    /// Returns a reference to the shard task if this is a Shard variant
    pub fn as_shard(&self) -> Option<&ShardTask> {
        match self {
            Task::Shard(task) => Some(task),
            _ => None,
        }
    }

    /// Returns a reference to the compose task if this is a Compose variant
    pub fn as_compose(&self) -> Option<&ComposeTask> {
        match self {
            Task::Compose(task) => Some(task),
            _ => None,
        }
    }

    /// Returns a reference to the recursion CPU task if this is a RecursionCpu variant
    pub fn as_recursion_cpu(&self) -> Option<&RecursionCpuTask> {
        match self {
            Task::RecursionCpu(task) => Some(task),
            _ => None,
        }
    }

    /// Returns a reference to the recursion GPU task if this is a RecursionGpu variant
    pub fn as_recursion_gpu(&self) -> Option<&RecursionGpuTask> {
        match self {
            Task::RecursionGpu(task) => Some(task),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskType {
    #[serde(rename = "core_shard_checkpoint")]
    CoreShardCheckpoint,
    #[serde(rename = "special_checkpoint")]
    SpecialCheckpoint,
    #[serde(rename = "shard")]
    Shard,
    #[serde(rename = "compose")]
    Compose,
    #[serde(rename = "recursion_cpu")]
    RecursionCpu,
    #[serde(rename = "recursion_gpu")]
    RecursionGpu,
}

impl fmt::Display for TaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskType::CoreShardCheckpoint => write!(f, "core_shard_checkpoint"),
            TaskType::SpecialCheckpoint => write!(f, "special_checkpoint"),
            TaskType::Shard => write!(f, "shard"),
            TaskType::Compose => write!(f, "compose"),
            TaskType::RecursionCpu => write!(f, "recursion_cpu"),
            TaskType::RecursionGpu => write!(f, "recursion_gpu"),
        }
    }
}
