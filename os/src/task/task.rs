//! Types related to task management

use log::debug;

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{
        memory_set::{MapPermission, MemorySet, KERNEL_SPACE},
        PhysPageNum, VirtAddr,
    },
    trap::{trap_handler, TrapContext},
};

use super::TaskContext;

pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(app_id: usize, data: &[u8]) -> Self {
        let (memory_set, user_stack_top, entry) = MemorySet::from_elf(data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        debug!(
            "app {}, kernel stack: [{:#x?} - {:#x?})",
            app_id, kernel_stack_bottom, kernel_stack_top
        );

        // we can use kernel space as identical or framed mapping, they are both ok
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        let task_status = TaskStatus::Ready;
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);

        let trap_cx = unsafe { trap_cx_ppn.get_mut::<TrapContext>() };
        *trap_cx = TrapContext::app_init_context(
            entry,
            user_stack_top.into(),
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as _,
        );

        Self {
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            base_size: user_stack_top.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}
