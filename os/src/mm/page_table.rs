//! page table

use core::ops::Add;

use alloc::vec::Vec;
use bitflags::*;

use crate::{
    config::{PAGE_SIZE, TRAMPOLINE},
    mm::{PhysAddr, VirtAddr},
};

use super::{
    address::{PhysPageNum, VirtPageNum},
    frame_allocator::{frame_alloc, FrameTracker},
    memory_set::MapPermission,
};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

impl PTEFlags {
    pub fn to_permission_flags(&self) -> MapPermission {
        let mut ret = MapPermission::empty();
        if self.contains(PTEFlags::R) {
            ret |= MapPermission::R;
        }
        if self.contains(PTEFlags::W) {
            ret |= MapPermission::W;
        }
        if self.contains(PTEFlags::X) {
            ret |= MapPermission::X;
        }
        if self.contains(PTEFlags::U) {
            ret |= MapPermission::U;
        }
        ret
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        Self(ppn.0 << 10 | flags.bits as usize)
    }

    pub fn ppn(&self) -> PhysPageNum {
        (self.0 >> 10 & ((1usize << 44) - 1)).into()
    }

    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.0 as u8).unwrap()
    }

    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        let root_ppn = frame.ppn;
        Self {
            root_ppn,
            frames: vec![frame],
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.root_ppn = PhysPageNum(0);
    }

    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

impl PageTable {
    pub fn find_pte_or_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for (idx, &pte_idx) in indexes.iter().enumerate() {
            let pte_array = unsafe { ppn.get_pte_array() };
            let pte = pte_array.get_mut(pte_idx).unwrap();
            if idx == indexes.len() - 1 {
                return Some(pte);
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        None
    }

    pub fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let indexes = vpn.indexes();
        let mut ppn = self.root_ppn;
        for (idx, &pte_idx) in indexes.iter().enumerate() {
            let pte = unsafe { ppn.get_pte_array() }.get_mut(pte_idx).unwrap();
            if idx == indexes.len() - 1 {
                return Some(pte);
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        None
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).cloned()
    }
}

impl PageTable {
    /// build the map relationship between vpn and ppn, the actual page frame of ppn should be allocated before
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_or_create(vpn).unwrap();
        assert!(
            !pte.is_valid(),
            "VPN({:#?}) to PPN({:#x?}) has been mapped",
            vpn,
            ppn
        );
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    /// unmap the map relationship between vpn and ppn, the actual page frame of ppn should be owned by the caller
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "VPN({:#?}) has not been mapped", vpn);
        pte.clear();
    }
}

pub struct BorrowedPageTable {
    inner: PageTable,
}

impl BorrowedPageTable {
    // create a page table from a user satp token
    // but the actual frames are stored in the real page table
    pub fn from_token(satp: usize) -> Self {
        Self {
            inner: PageTable {
                root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
                frames: vec![],
            },
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.inner.translate(vpn)
    }
}

pub fn page_table_test() {
    let start_vpn = 0b000000001_000000011_000000000;
    // indexes: [1, 3, 0] ~ [1, 3, 4]
    let vpns = (start_vpn..(start_vpn + 5))
        .map(VirtPageNum)
        .collect::<Vec<_>>();
    let mut frames: Vec<FrameTracker> = vec![];

    let mut page_table = PageTable::new();
    for vpn in &vpns {
        let frame = frame_alloc().unwrap();
        page_table.map(*vpn, frame.ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W);
        frames.push(frame);
    }
    // there are 3 frames as directories, level2, level1, level0
    assert_eq!(page_table.frames.len(), 3);

    let start_vpn2 = 0b000000001_000000111_000000000;

    let vpns2 = (start_vpn2..(start_vpn2 + 5))
        .map(VirtPageNum)
        .collect::<Vec<_>>();
    for vpn in &vpns2 {
        let frame = frame_alloc().unwrap();
        page_table.map(*vpn, frame.ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W);
        frames.push(frame);
    }
    // add 1 more frame at directory level1
    assert_eq!(page_table.frames.len(), 4);

    let non_exist_vpn = VirtPageNum(0b000000001_000001111_000000000);

    assert_eq!(
        page_table.translate(vpns.last().unwrap().add(1)),
        Some(PageTableEntry::default())
    );
    assert!(page_table
        .translate(vpns.last().cloned().unwrap())
        .unwrap()
        .is_valid(),);
    assert!(page_table.translate(non_exist_vpn).is_none());

    extern "C" {
        fn strampoline();
    }
    page_table.map(
        VirtAddr(TRAMPOLINE).into(),
        PhysAddr(strampoline as _).into(),
        PTEFlags::R | PTEFlags::X,
    );
    assert!(page_table.translate(VirtAddr(TRAMPOLINE).into()).is_some());
    assert!(page_table
        .translate(VirtAddr(TRAMPOLINE - PAGE_SIZE).into())
        .is_some());

    println!("page_table_test passed!");
}
