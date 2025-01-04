use alloc::vec::Vec;
use lazy_static::lazy_static;

use crate::{
    config::kernel_stack_position,
    mm::memory_set::{MapPermission, KERNEL_SPACE},
    sync::UPSafeCell,
};

lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> =
        unsafe { UPSafeCell::new(PidAllocator::new()) };
}

pub fn alloc_pid() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}

#[derive(Debug, PartialEq)]
pub struct PidHandle(usize);

impl AsRef<usize> for PidHandle {
    fn as_ref(&self) -> &usize {
        &self.0
    }
}

impl Drop for PidHandle {
    fn drop(&mut self) {
        PID_ALLOCATOR.exclusive_access().dealloc(self.0);
    }
}

struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        PidAllocator {
            current: 0,
            recycled: Vec::new(),
        }
    }
    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

pub struct KernelStack {
    pub pid: usize,
}

impl KernelStack {
    pub fn new(pid: usize) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);

        // we can use kernel space as identical or framed mapping, they are both ok
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );

        Self { pid }
    }

    pub fn get_sp(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top.into()
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(self.pid);
        KERNEL_SPACE.exclusive_access().remove_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
        );
    }
}
