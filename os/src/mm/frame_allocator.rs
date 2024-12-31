//! frame allocator

use core::fmt;
use super::*;

use alloc::{boxed::Box, vec::Vec};
use log::debug;

use crate::{board::MEMORY_END, sync::UPSafeCell};

use super::address::PhysPageNum;

type BoxedFrameAllocator = Box<dyn FrameAllocator + Send>;

lazy_static::lazy_static! {
    static ref FRAME_ALLOCATOR: UPSafeCell<BoxedFrameAllocator> = unsafe { UPSafeCell::new(StackFrameAllocator::new_boxed()) };
}

pub fn init_frame_allocator() {
    extern "C" {
        fn ekernel();
    }
    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

pub fn frame_dealloc(frame: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(frame);
}

pub trait FrameAllocator {
    fn init(&mut self, start: PhysPageNum, end: PhysPageNum);
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_array = unsafe { ppn.get_bytes_array() };
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }

    pub fn ppn(&self) -> PhysPageNum {
        self.ppn
    }
}

impl fmt::Debug for FrameTracker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FrameTracker:PPN={:#x}", self.ppn().0)
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn());
    }
}

/// [start, end) is the usable range of physical memory for page frame allocation
pub struct StackFrameAllocator {
    start: PhysPageNum,
    end: PhysPageNum,
    recycled: Vec<PhysPageNum>,
}

impl Default for StackFrameAllocator {
    fn default() -> Self {
        Self {
            start: PhysPageNum(0),
            end: PhysPageNum(0),
            recycled: Vec::new(),
        }
    }
}

impl StackFrameAllocator {
    pub fn new_boxed() -> BoxedFrameAllocator {
        Box::new(Self::default())
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn)
        } else if self.start == self.end {
            None
        } else {
            let ppn = self.start;
            self.start = self.start + 1;
            Some(ppn)
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        if ppn >= self.start || self.recycled.iter().any(|p| *p == ppn) {
            panic!("Frame {:?} has not been alloced", ppn);
        }
        self.recycled.push(ppn);
    }

    fn init(&mut self, start: PhysPageNum, end: PhysPageNum) {
        self.start = start;
        self.end = end;
    }
}

#[allow(unused)]
/// a simple test for frame allocator
pub fn frame_allocator_test() {
    extern "C" {
        fn ekernel();
    }
    let start_ppn: PhysPageNum = (PhysAddr::from(ekernel as usize)).into();

    let mut v: Vec<FrameTracker> = Vec::new();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        if i == 0 {
            assert!(frame.ppn() == start_ppn);
        }
        debug!("{:?}", frame);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        debug!("{:?}", frame);
        if i == 4 {
            assert!(frame.ppn() == start_ppn);
        }
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
