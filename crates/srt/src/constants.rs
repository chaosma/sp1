use std::time::Duration;

/// Constants for storage paths
pub const SHARED_RAMDISK_PATH: &str = "/mnt/shared_ramdisk/srt";
// Used to store small files in local
pub const LOCAL_RAMDISK_PATH: &str = "/mnt/local_ramdisk/srt";
// Used to store large files in local
pub const LOCAL_HUGE_TLB_PATH: &str = "/mnt/hugetlbfs/srt";

pub const TASK_FIELD_KEY: &str = "task";
pub const SRT_GROUP_NAME: &str = "srt";
pub const DEFAULT_STREAM_BLOCK_MS: u64 = 3000;

/// Default TTL for Redis keys (6 hours)
pub const DEFAULT_REDIS_TTL: Duration = Duration::from_secs(6 * 60 * 60);
pub const DEFAULT_REDIS_HOST: &str = "localhost";
pub const DEFAULT_REDIS_PORT: u16 = 6379;

// TODO: Don't hardcode the program path. Ideally, we should link program id to the task id (root_id) in the o10r task
pub const ETH_BLOCK_PROGRAM_PATH: &str = "/mnt/shared_ramdisk/srt/program/eth-block";
