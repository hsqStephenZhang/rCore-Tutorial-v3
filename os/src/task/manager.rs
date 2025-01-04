use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use super::*;

#[derive(Default)]
pub struct TaskManager {
    queue: VecDeque<Arc<TaskControlBlock>>,
}

lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> = unsafe {
        UPSafeCell::new(TaskManager::default())
    };
}

impl TaskManager {
    pub fn add_task(&mut self, tcb: Arc<TaskControlBlock>) {
        self.queue.push_back(tcb);
    }

    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

pub fn add_task(tcb: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add_task(tcb);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}

pub fn is_rq_empty() -> bool {
    TASK_MANAGER.exclusive_access().is_empty()
}
