pub mod context;
pub mod switch;
pub mod task;

use context::TaskContext;
use switch::__switch;

use crate::{
    config::MAX_APP_NUM,
    loader::{init_app_ctx, load_apps},
    sbi::shutdown,
    sync::UPUnsafeCell,
};

pub struct TaskManager {
    num_tasks: usize,
    inner: UPUnsafeCell<TaskManagerInner>,
}

pub struct TaskManagerInner {
    current_task: usize,
    tasks: [task::TaskControlBlock; MAX_APP_NUM],
}

lazy_static::lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = unsafe {
        let mut inner = TaskManagerInner {
            current_task: 0,
            tasks: [task::TaskControlBlock::default(); MAX_APP_NUM],
        };
        load_apps();
        // safety: `load_apps` has been called
        let num_app = crate::loader::get_app_num();
        for i in 0..num_app {
            let task = &mut inner.tasks[i];
            task.context = TaskContext::goto_restore_with_kernel_stack(
                init_app_ctx(i)
            );
            task.status = task::TaskStatus::Ready;
        }

        TaskManager {
            num_tasks: num_app,
            inner: UPUnsafeCell::new(inner),
        }
    };
}

impl TaskManager {
    pub fn num_tasks(&self) -> usize {
        self.num_tasks
    }

    pub fn current_task(&self) -> usize {
        self.inner.borrow_mut().current_task
    }

    pub fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.borrow_mut();
        (inner.current_task + 1..=inner.current_task + self.num_tasks)
            .map(|i| i % self.num_tasks)
            .find(|i| inner.tasks[*i].status == task::TaskStatus::Ready)
    }

    pub fn run_first_task(&self) {
        // create an empty current task context
        let current_task_cx = TaskContext::default();
        let current_task_cx_ptr = &current_task_cx as *const _ as *mut _;
        let mut inner = self.inner.borrow_mut();
        inner.tasks[0].status = task::TaskStatus::Running;
        let next_task_cx_ptr = &inner.tasks[0].context as *const _;
        drop(inner);
        unsafe {
            __switch(current_task_cx_ptr, next_task_cx_ptr);
        }
        panic!("unreachable after __switch");
    }

    fn run_next_task(&self) {
        if let Some(next_task) = self.find_next_task() {
            let mut inner = self.inner.borrow_mut();
            let current_task = inner.current_task;
            let current_task_cx_ptr = &inner.tasks[current_task].context as *const _ as *mut _;
            let next_task_cx_ptr = &inner.tasks[next_task].context as *const _;
            inner.current_task = next_task;
            drop(inner);
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            println!("all app has been loaded, shutdown");
            shutdown(false);
        }
    }

    // suspend the current task and run the next
    fn suspend_and_run_next(&self) {
        let mut inner = self.inner.borrow_mut();
        let current_task = inner.current_task;
        inner.tasks[current_task].status = task::TaskStatus::Ready;
        drop(inner);
        self.run_next_task();
    }

    /// exit the current task and run the next
    fn exit_and_run_next(&self) {
        let mut inner: core::cell::RefMut<'_, TaskManagerInner> = self.inner.borrow_mut();
        let current_task = inner.current_task;
        inner.tasks[current_task].status = task::TaskStatus::Finished;
        drop(inner);
        self.run_next_task();
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn exit_current_and_run_next() {
    TASK_MANAGER.exit_and_run_next();
}

pub fn suspend_current_and_run_next() {
    TASK_MANAGER.suspend_and_run_next();
}

pub fn current_task() -> usize {
    TASK_MANAGER.current_task()
}
