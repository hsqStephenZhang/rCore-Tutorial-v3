use crate::{sbi::shutdown, sync::UPSafeCell, trap::TrapContext};

use super::{manager::{fetch_task, is_rq_empty}, switch::__switch, task::{TaskControlBlock, TaskStatus}, TaskContext};
use alloc::sync::Arc;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

pub struct Processor {
    pub task: Option<Arc<TaskControlBlock>>,
    pub idle_task_cx: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            task: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.task.clone()
    }

    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.task.take()
    }
}

pub fn run_tasks() {
    loop {
        if is_rq_empty() {
            log::info!("run_tasks: no task to run");
            shutdown(false);
        }
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let mut inner = task.inner();
            inner.task_status = TaskStatus::Running;
            let next_task_cx_ptr = &mut inner.task_cx as *const TaskContext;
            drop(inner);
            processor.task = Some(task);
            let current_task_cx_ptr = &mut processor.idle_task_cx as *mut TaskContext;
            drop(processor);
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}

// save current task context, and switch to `run_tasks` and execute the next loop
pub fn schedule(task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = &mut processor.idle_task_cx as *mut TaskContext;
    drop(processor);
    unsafe {
        __switch(task_cx_ptr, idle_task_cx_ptr);
    }
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    let processor = PROCESSOR.exclusive_access();
    processor.current()
}

pub fn take_current() -> Option<Arc<TaskControlBlock>> {
    let mut processor = PROCESSOR.exclusive_access();
    processor.take_current()
}

#[no_mangle]
pub fn current_user_token() -> usize {
    let processor = PROCESSOR.exclusive_access();
    let task = processor.task.as_ref().unwrap();
    let inner = task.inner();
    inner.memory_set.token()
}
export_func_simple!(current_user_token);

#[no_mangle]
pub fn current_trap_cx() -> &'static mut TrapContext {
    let processor = PROCESSOR.exclusive_access();
    let task = processor.task.as_ref().unwrap();
    let inner = task.inner();
    let trap_cx_ppn = inner.trap_cx_ppn;
    unsafe { trap_cx_ppn.get_mut::<TrapContext>() }
}
export_func_simple!(current_trap_cx);