//! Types related to task management
use crate::config::MAX_SYSCALL_NUM;

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    
    /// first_time
    pub task_first_time : usize,
}

#[derive(Clone, Copy)]
pub struct TaskSysCall{
    pub data : [u32;MAX_SYSCALL_NUM],
}
impl TaskSysCall {
    pub fn new()->Self{
        TaskSysCall  { data: [0;MAX_SYSCALL_NUM]}
    }
}


/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
