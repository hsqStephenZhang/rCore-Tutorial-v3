//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod manager;
mod pid;
mod processor;
mod switch;

#[allow(clippy::module_inception)]
mod task;

use crate::loader::get_app_data_by_name;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use lazy_static::*;
pub use manager::{add_task, fetch_task, TASK_MANAGER};
pub use pid::*;
pub use processor::*;
pub use task::{
    mmap_current_task, munmap_current_task, sbrk_current_task, TaskControlBlock, TaskStatus,
};

pub use context::TaskContext;

lazy_static! {
    pub static ref INIT_PROCESS: UPSafeCell<Arc<TaskControlBlock>> = {
        let data = get_app_data_by_name("initproc").unwrap();
        let task = TaskControlBlock::new(data);
        unsafe { UPSafeCell::new(Arc::new(task)) }
    };
}

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    let task = take_current().unwrap();
    let mut inner = task.inner();
    inner.task_status = TaskStatus::Ready;
    let current_task_cx = &mut inner.task_cx as *mut _;
    drop(inner);
    TASK_MANAGER.exclusive_access().add_task(task);
    schedule(current_task_cx);
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next(exit_code: isize) {
    let cur_task = take_current().unwrap();
    let mut inner = cur_task.inner();
    inner.exit_code = exit_code;
    inner.task_status = TaskStatus::Zombie;

    // orphan current task's children
    {
        let init_proc = INIT_PROCESS.exclusive_access();
        for child in inner.children.iter() {
            child.inner().parent = Some(Arc::downgrade(&init_proc));
            init_proc.inner().children.push(child.clone());
        }
    }

    inner.children.clear();
    inner.memory_set.clear_pages();

    drop(inner);
    drop(cur_task);
    let mut _zero_cx = TaskContext::zero_init();
    schedule(&mut _zero_cx as *mut _);
}

pub fn init() {
    add_task(INIT_PROCESS.exclusive_access().clone());
}
