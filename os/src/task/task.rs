use super::context::TaskContext;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TaskStatus {
    #[default]
    UnInit,
    Ready,
    Running,
    Finished,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskControlBlock {
    pub context: TaskContext,
    pub status: TaskStatus,
}
