//! Types related to task management

use log::debug;

use crate::{
    config::{kernel_stack_position, TRAP_CONTEXT},
    mm::{
        memory_set::{MapPermission, MemorySet, KERNEL_SPACE},
        PhysPageNum, VPNRange, VirtAddr,
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
    pub heap_bottom: usize,
    pub heap_brk: usize,
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
            heap_bottom: user_stack_top.into(),
            heap_brk: user_stack_top.into(),
        }
    }

    pub fn sbrk(&mut self, new_brk_size: i32) -> Option<usize> {
        let new_brk = (self.heap_brk as i32 + new_brk_size) as usize;
        if new_brk < self.heap_bottom {
            return None;
        }
        let origin_brk = self.heap_brk;
        if new_brk > self.heap_brk {
            self.memory_set
                .append_to(self.heap_bottom.into(), new_brk.into());
        } else if new_brk < self.heap_brk {
            self.memory_set
                .shrink_to(self.heap_bottom.into(), new_brk.into());
        }
        self.heap_brk = new_brk;
        Some(origin_brk)
    }

    pub fn mmap(&mut self, addr: usize, size: usize, perm: MapPermission) -> isize {
        let start_va = VirtAddr::from(addr);
        let end_va = VirtAddr::from(addr + size);

        debug!(
            "mmap: start_va: {:?}, end_va: {:?}, perm: {:?}",
            start_va, end_va, perm
        );
        let vpn_range = VPNRange::from_addr_range(start_va, end_va);
        for vpn in vpn_range {
            let pte = self.memory_set.translate(vpn.into());
            if let Some(pte) = pte {
                if pte.is_valid() {
                    return -1;
                }
            }
        }

        self.memory_set.insert_framed_area(start_va, end_va, perm);
        0
    }

    pub fn munmap(&mut self, start: usize, size: usize) -> isize {
        let start_va = VirtAddr::from(start);
        let end_va = VirtAddr::from(start + size);

        // debug!(
        //     "munmap: start_va: {:?}, end_va: {:?}",
        //     start_va, end_va
        // );
        let vpn_range = VPNRange::from_addr_range(start_va, end_va);
        for vpn in vpn_range {
            match self.memory_set.translate(vpn.into()) {
                Some(pte) => {
                    if !pte.is_valid() {
                        return -1;
                    }
                }
                None => {
                    return -1;
                }
            }
        }
        let page_table = self.memory_set.get_page_table();
        for vpn in vpn_range {
            page_table.unmap(vpn);
        }

        // self.memory_set.remove_area(start_va, end_va);
        0
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
}
