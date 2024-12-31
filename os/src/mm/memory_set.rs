//! memory area and memory set

use core::{arch::asm, fmt};

use crate::{
    board::MEMORY_END,
    config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE},
    loader::{get_app_data, get_num_app},
    sync::UPSafeCell,
};

use super::page_table::PageTable;
use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use bitflags::bitflags;
use log::{debug, info, trace};
use page_table::{PTEFlags, PageTableEntry};
use riscv::register::satp;

use super::*;

lazy_static::lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe{
        UPSafeCell::new(MemorySet::new_kernel())
    });
}

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl fmt::Debug for MapArea {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapArea")
            .field("vpn_range", &self.vpn_range)
            .field("map_type", &self.map_type)
            .field("map_perm", &self.map_perm)
            .finish()
    }
}

impl MapArea {
    pub fn new(
        start_addr: VirtAddr,
        end_addr: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let vpn_range = VPNRange::from_addr_range(start_addr, end_addr);
        Self {
            vpn_range,
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range.into_iter() {
            self.map_one(page_table, vpn);
        }
    }

    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range.into_iter() {
            self.unmap_one(page_table, vpn);
        }
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
            self.unmap_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }
    #[allow(unused)]
    pub fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
        for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
            self.map_one(page_table, vpn)
        }
        self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
    }

    // SAFETY: call after map(alloc physical frames to store the data) && is Framed
    pub unsafe fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert!(self.map_type == MapType::Framed);
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        let mut start = 0;
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array();
            dst[..src.len()].copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

impl MapArea {
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn();
                self.data_frames.insert(vpn, frame);
            }
        }
        page_table.map(vpn, ppn, PTEFlags::from_bits(self.map_perm.bits()).unwrap());
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Identical => {}
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
        }
        page_table.unmap(vpn);
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl fmt::Debug for MemorySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MemorySet")
            .field("areas", &self.areas)
            .finish()
    }
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }

    pub fn get_page_table(&mut self) -> &mut PageTable {
        &mut self.page_table
    }

    fn push(&mut self, mut area: MapArea, data: Option<&[u8]>) {
        area.map(&mut self.page_table);
        if let Some(data) = data {
            // now we have mapped the area, it's safe to copy data
            unsafe {
                area.copy_data(&self.page_table, data);
            }
        }
        self.areas.push(area);
    }

    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va, MapType::Framed, permission),
            None,
        );
    }

    #[allow(unused)]
    pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            trace!(
                "shrink to, origin end: {:?}, new end: {:?}",
                area.vpn_range.get_end(),
                new_end.ceil()
            );
            area.shrink_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }
    #[allow(unused)]
    pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.get_start() == start.floor())
        {
            trace!(
                "append to, origin end: {:?}, new end: {:?}",
                area.vpn_range.get_end(),
                new_end.ceil()
            );
            area.append_to(&mut self.page_table, new_end.ceil());
            true
        } else {
            false
        }
    }

    pub fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr(TRAMPOLINE).into(),
            PhysAddr(strampoline as _).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }
}

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

impl MemorySet {
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();

        info!(".text: [{:x}, {:x})", stext as usize, etext as usize);
        info!(".rodata: [{:x}, {:x})", srodata as usize, erodata as usize);
        info!(".data: [{:x}, {:x})", sdata as usize, edata as usize);
        info!(
            ".bss: [{:x}, {:x})",
            sbss_with_stack as usize, ebss as usize
        );

        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                MapType::Identical,
                MapPermission::R,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        memory_set
    }

    // return memory set, user stack top address, entry point of user program
    pub fn from_elf(elf_data: &[u8]) -> (Self, VirtAddr, usize) {
        let mut memory_set = Self::new_bare();
        memory_set.map_trampoline();

        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        assert_eq!(elf_header.pt1.magic, xmas_elf::header::MAGIC);
        debug!("memory set from elf");
        let mut max_end_vpn = VirtPageNum(0);
        for ph in elf.program_iter() {
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va = VirtAddr(ph.virtual_addr() as usize);
                let end_va = VirtAddr(ph.virtual_addr() as usize + ph.mem_size() as usize);
                let mut permission = MapPermission::U;
                if ph.flags().is_execute() {
                    permission |= MapPermission::X;
                }
                if ph.flags().is_read() {
                    permission |= MapPermission::R;
                }
                if ph.flags().is_write() {
                    permission |= MapPermission::W;
                }
                let data =
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]);
                debug!(
                    "map [0x{:x}, 0x{:x}), permission: {:?}, data size: 0x{:x}",
                    start_va.0,
                    end_va.0,
                    permission,
                    data.as_ref().map(|x| x.len()).unwrap_or(0)
                );
                let area = MapArea::new(start_va, end_va, MapType::Framed, permission);
                max_end_vpn = area.vpn_range.get_end();
                memory_set.push(area, data)
            }
        }

        // protect page & user_stack
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        debug!(
            "map user stack: [{:x}, {:x})",
            user_stack_bottom, user_stack_top
        );
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        debug!("map heap : [{:x}, {:x})", user_stack_top, user_stack_top);
        memory_set.push(
            MapArea::new(
                user_stack_top.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        debug!("map trap context: [{:x}, {:x})", TRAP_CONTEXT, TRAMPOLINE);
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        (
            memory_set,
            user_stack_top.into(),
            elf_header.pt2.entry_point() as _,
        )
    }
}

impl MemorySet {
    pub fn activate(&self) {
        let token = self.page_table.token();
        unsafe {
            satp::write(token);
            asm!("sfence.vma");
        }
    }
}

pub fn remap_test() {
    let kernel_space = KERNEL_SPACE.exclusive_access();
    // it's identically mapped, so we can take physical address as virtual address
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    let end_memory: VirtAddr = MEMORY_END.into();
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .flags()
            .to_permission_flags(),
        MapPermission::R | MapPermission::X
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .flags()
            .to_permission_flags(),
        MapPermission::R,
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .flags()
            .to_permission_flags(),
        MapPermission::R | MapPermission::W,
    );
    assert!(kernel_space
        .page_table
        .translate(end_memory.floor())
        .is_none());

    debug!("remap_test passed!");
}

pub fn load_user_apps_test() {
    let num_apps = get_num_app();
    for i in 0..num_apps {
        let app_data = get_app_data(i);
        let (app_memory_set, user_stack_top, _entry) = MemorySet::from_elf(app_data);
        let user_stack_bottom: VirtAddr = (user_stack_top.0 - USER_STACK_SIZE).into();
        let protect_page: VirtAddr = (user_stack_bottom.0 - PAGE_SIZE).into();
        assert_eq!(
            app_memory_set
                .page_table
                .translate(user_stack_bottom.floor())
                .unwrap()
                .flags()
                .to_permission_flags(),
            MapPermission::R | MapPermission::W | MapPermission::U,
        );

        assert_eq!(
            app_memory_set
                .page_table
                .translate(protect_page.floor())
                .unwrap()
                .flags()
                .to_permission_flags(),
            MapPermission::empty(),
        );
        assert!(app_memory_set
            .page_table
            .translate(VirtAddr(TRAMPOLINE).into())
            .is_some());
        assert_eq!(
            app_memory_set
                .page_table
                .translate(VirtAddr(TRAP_CONTEXT - PAGE_SIZE).into()),
            Some(PageTableEntry::default())
        );
    }
}
