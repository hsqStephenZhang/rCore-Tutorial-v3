//! Types related to task management

use alloc::{
    borrow::ToOwned,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use log::{debug, trace};

use crate::{
    config::TRAP_CONTEXT,
    loader::get_app_data_by_name,
    mm::{
        memory_set::{MapPermission, MemorySet, KERNEL_SPACE},
        PhysPageNum, VPNRange, VirtAddr,
    },
    sync::UPSafeCell,
    task::alloc_pid,
    trap::{trap_handler, TrapContext},
};

use super::{current_task, pid::PidHandle, processor::PROCESSOR, KernelStack, TaskContext};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Exited,
    Zombie,
}

pub struct TaskControlBlock {
    pub pid: PidHandle,
    kernel_stack: KernelStack,
    inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
    pub fn getpid(&self) -> usize {
        *self.pid.as_ref()
    }

    pub fn get_cmdline(&self) -> Option<String> {
        self.inner().cmdline.clone()
    }

    pub fn status(&self) -> TaskStatus {
        self.inner().task_status
    }

    pub fn exit_code(&self) -> isize {
        self.inner().exit_code
    }
}

impl Drop for TaskControlBlock {
    fn drop(&mut self) {
        trace!("TCB drop: {:?}", self.pid);
    }
}

pub struct TaskControlBlockInner {
    pub cmdline: Option<String>,
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: isize,
    pub base_size: usize,
    pub heap_bottom: usize,
    pub heap_brk: usize,
}

impl TaskControlBlockInner {
    pub fn get_trap_cx(&self) -> &mut TrapContext {
        unsafe { self.trap_cx_ppn.get_mut::<TrapContext>() }
    }
}

impl TaskControlBlock {
    pub fn new(cmdline: &str) -> Self {
        let pid = alloc_pid();
        let data = get_app_data_by_name(cmdline).unwrap();
        let (memory_set, user_stack_top, entry) = MemorySet::from_elf(data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let kernel_stack = KernelStack::new(*pid.as_ref());
        let kernel_stack_top = kernel_stack.get_sp();

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

        let inner = TaskControlBlockInner {
            cmdline: Some(cmdline.to_owned()),
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            children: Vec::new(),
            parent: None,
            exit_code: 0,
            base_size: data.len(),
            heap_bottom: 0,
            heap_brk: 0,
        };
        Self {
            pid,
            kernel_stack,
            inner: unsafe { UPSafeCell::new(inner) },
        }
    }

    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        let mut parent_inner = self.inner();

        let pid = alloc_pid();
        let memory_set = MemorySet::from_another(&parent_inner.memory_set);

        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();

        let kernel_stack = KernelStack::new(*pid.as_ref());
        let kernel_stack_top = kernel_stack.get_sp();

        let task_status = TaskStatus::Ready;
        let task_cx = TaskContext::goto_trap_return(kernel_stack_top);

        let inner = TaskControlBlockInner {
            cmdline: parent_inner.cmdline.clone(),
            task_status,
            task_cx,
            memory_set,
            trap_cx_ppn,
            parent: Some(Arc::downgrade(self)),
            exit_code: 0,
            children: Vec::new(),
            base_size: parent_inner.base_size,
            heap_bottom: 0,
            heap_brk: 0,
        };
        let new_task = Arc::new(Self {
            pid,
            kernel_stack,
            inner: unsafe { UPSafeCell::new(inner) },
        });

        let new_inner = new_task.inner();
        let trap_cx = new_inner.get_trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;
        drop(new_inner);

        parent_inner.children.push(new_task.clone());

        new_task
    }

    pub fn exec(self: &Arc<Self>, path: &str) -> Result<(), ()> {
        let data = match get_app_data_by_name(path) {
            Some(data) => data,
            None => return Err(()),
        };
        let (memory_set, user_stack_top, entry) = MemorySet::from_elf(data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let mut inner = self.inner();
        inner.trap_cx_ppn = trap_cx_ppn;
        inner.memory_set = memory_set;

        let trap_cx = unsafe { inner.trap_cx_ppn.get_mut::<TrapContext>() };
        *trap_cx = TrapContext::app_init_context(
            entry,
            user_stack_top.into(),
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_sp(),
            trap_handler as _,
        );

        inner.cmdline = Some(path.to_owned());
        inner.base_size = data.len();
        inner.heap_bottom = 0;
        inner.heap_brk = 0;
        debug!("exec: pid = {}, entry = {:#x}", self.getpid(), entry);
        Ok(())
    }

    pub fn inner(&self) -> core::cell::RefMut<'_, TaskControlBlockInner> {
        self.inner.exclusive_access()
    }
}

impl TaskControlBlockInner {
    /// memory
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

pub fn sbrk_current_task(new_brk_size: i32) -> Option<usize> {
    let processor = PROCESSOR.exclusive_access();
    let task = processor.task.as_ref().unwrap();
    let mut inner = task.inner();
    inner.sbrk(new_brk_size)
}

pub fn mmap_current_task(start: usize, len: usize, protection: usize) -> isize {
    let mut permission = MapPermission::U;
    if protection & 0b1 != 0 {
        permission |= MapPermission::R;
    }
    if protection & 0b10 != 0 {
        permission |= MapPermission::W;
    }
    if protection & 0b100 != 0 {
        permission |= MapPermission::X;
    }
    let task = current_task().unwrap();
    let mut inner = task.inner();
    inner.mmap(start, len, permission)
}

pub fn munmap_current_task(start: usize, len: usize) -> isize {
    let task = current_task().unwrap();
    let mut inner = task.inner();
    inner.munmap(start, len)
}
