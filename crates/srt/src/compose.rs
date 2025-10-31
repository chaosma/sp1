use crate::task::{ComposeTask, RecursionCpuTask, Task};

/// Calculates the total number of levels in the recursion tree for a given number of shards.
///
/// # Arguments
/// * `total_shards` - The total number of shards to be processed
///
/// # Returns
/// The number of levels needed in the recursion tree, calculated as log2(total_shards) rounded up + 1
pub fn calculate_total_levels(total_shards: u32) -> u32 {
    (f64::from(total_shards)).log2().ceil() as u32 + 1
}

/// Check if the compose task is complete.
///
/// # Arguments
/// * `task` - The compose task to check
/// * `total_shards` - The total number of shards
///
/// # Returns
/// True if the compose task is complete, false otherwise.
pub fn is_complete(task: &ComposeTask, total_shards: u32) -> bool {
    calculate_total_levels(total_shards) == task.height + 1
}

/// Get the paired compose task for the given task.
///
/// # Arguments
/// * `task` - The compose task to get the paired task for
///
/// # Returns
/// The paired compose task
pub fn get_paired_compose_task(task: &ComposeTask) -> ComposeTask {
    if task.index % 2 == 0 {
        ComposeTask { root_id: task.root_id.clone(), height: task.height, index: task.index + 1 }
    } else {
        ComposeTask { root_id: task.root_id.clone(), height: task.height, index: task.index - 1 }
    }
}

/// Get the next level recursion task for the given task.
///
/// # Arguments
/// * `task` - The compose task to get the next level recursion task for
/// * `total_shards` - The total number of shards
///
/// # Returns
/// The next level recursion task
pub fn get_next_level_recursion_task(task: &ComposeTask, total_shards: Option<u32>) -> Task {
    Task::RecursionCpu(RecursionCpuTask {
        root_id: task.root_id.clone(),
        height: task.height + 1,
        index: task.index / 2,
        total_shards,
    })
}

/// Get the equivalent compose task for the given task. Figure out the equivalent position in the
/// recursion tree if the compose task is not paird at current level.
///
/// # Arguments
/// * `task` - The compose task to get the equivalent task for
/// * `total_shards` - The total number of shards
///
/// # Returns
/// The equivalent compose task if it exists, otherwise None.
pub fn get_equivalent_compose_task(task: &ComposeTask, total_shards: u32) -> Option<ComposeTask> {
    let mut current_height = task.height;
    let mut current_level_nodes = (total_shards + (1 << current_height) - 1) >> current_height;

    // Special case: If it is the last node in current level and total number of nodes is odd,
    // figure out the position when it gets paired, then calculate the next node.
    if task.index + 1 == current_level_nodes && current_level_nodes % 2 == 1 {
        // Loop until this last node gets paired up
        while current_level_nodes % 2 == 1 {
            current_level_nodes = current_level_nodes / 2 + 1;
            current_height += 1;
        }
        // Return the equivalent task.
        Some(ComposeTask {
            root_id: task.root_id.clone(),
            height: current_height,
            index: current_level_nodes - 1,
        })
    } else {
        // No equivalent task found. Just return itself.
        None
    }
}

/// Orders compose tasks based on their position in the recursion tree.
///
/// Tasks are sorted by:
/// 1. Height (higher height comes first)
/// 2. Index (lower index comes first when heights are equal)
///
/// This ordering ensures that tasks are processed in the correct sequence
/// for the recursion tree traversal.
///
/// # Arguments
/// * `tasks` - Vector of compose tasks to be ordered
///
/// # Returns
/// A new vector containing the ordered tasks
pub fn order_compose_tasks(mut tasks: Vec<ComposeTask>) -> Vec<ComposeTask> {
    // Sort by height and index
    // If the heights are equal, small index comes first.
    // If the heights are not equals, higher height comes first.
    tasks.sort_unstable_by(|a, b| {
        if a.height != b.height {
            b.height.cmp(&a.height)
        } else {
            a.index.cmp(&b.index)
        }
    });
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_paired_compose_task_even() {
        let task = ComposeTask { root_id: "test".to_string(), height: 4, index: 6 };
        let paired_task = get_paired_compose_task(&task);
        assert_eq!(paired_task.root_id, task.root_id);
        assert_eq!(paired_task.height, task.height);
        assert_eq!(paired_task.index, task.index + 1);
    }

    #[test]
    fn test_get_paired_compose_task_odd() {
        let task = ComposeTask { root_id: "test".to_string(), height: 4, index: 7 };
        let paired_task = get_paired_compose_task(&task);
        assert_eq!(paired_task.root_id, task.root_id);
        assert_eq!(paired_task.height, task.height);
        assert_eq!(paired_task.index, task.index - 1);
    }

    #[test]
    fn test_get_next_level_recursion_task_odd() {
        let task = ComposeTask { root_id: "test".to_string(), height: 3, index: 3 };
        let next_task = get_next_level_recursion_task(&task, None);
        assert_eq!(next_task.as_recursion_cpu().unwrap().root_id, task.root_id);
        assert_eq!(next_task.as_recursion_cpu().unwrap().height, 4);
        assert_eq!(next_task.as_recursion_cpu().unwrap().index, 1);
        assert_eq!(next_task.as_recursion_cpu().unwrap().total_shards, None);
    }

    #[test]
    fn test_get_next_level_recursion_task_even() {
        let task = ComposeTask { root_id: "test".to_string(), height: 3, index: 4 };
        let next_task = get_next_level_recursion_task(&task, None);
        assert_eq!(next_task.as_recursion_cpu().unwrap().root_id, task.root_id);
        assert_eq!(next_task.as_recursion_cpu().unwrap().height, 4);
        assert_eq!(next_task.as_recursion_cpu().unwrap().index, 2);
        assert_eq!(next_task.as_recursion_cpu().unwrap().total_shards, None);
    }

    #[test]
    fn test_get_equivalent_compose_task_is_new() {
        let total_shards = 5;
        let task = ComposeTask { root_id: "test".to_string(), height: 0, index: 4 };

        let new_task = get_equivalent_compose_task(&task, total_shards);
        assert!(new_task.is_some());
        let new_task = new_task.unwrap();
        assert_eq!(new_task.root_id, task.root_id);
        assert_eq!(new_task.height, 2);
        assert_eq!(new_task.index, 1);
    }

    #[test]
    fn test_get_equivalent_compose_task_is_not_new() {
        let total_shards = 4;
        let task = ComposeTask { root_id: "test".to_string(), height: 0, index: 3 };

        let new_task = get_equivalent_compose_task(&task, total_shards);
        assert!(new_task.is_none());
    }

    #[test]
    fn test_order_compose_tasks() {
        let task1 = ComposeTask { root_id: "test".to_string(), height: 1, index: 1 };
        let task2 = ComposeTask { root_id: "test".to_string(), height: 1, index: 0 };
        let task3 = ComposeTask { root_id: "test".to_string(), height: 2, index: 0 };

        let tasks = vec![task1.clone(), task2.clone(), task3.clone()];
        let ordered = order_compose_tasks(tasks);

        // Higher height comes first
        assert_eq!(ordered[0], task3);
        // Same height, lower index comes first
        assert_eq!(ordered[1], task2);
        assert_eq!(ordered[2], task1);
    }
}
