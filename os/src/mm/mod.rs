//! memory management

pub mod address;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod memory_set;
pub mod page_table;

pub use address::*;
use alloc::vec::Vec;
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
use page_table::BorrowedPageTable;

pub fn init() {
    heap_allocator::init_heap();
    heap_allocator::heap_test();
    address::addr_test();
    frame_allocator::init_frame_allocator();
    frame_allocator::frame_allocator_test();
    page_table::page_table_test();
    memory_set::KERNEL_SPACE.exclusive_access().activate();
    memory_set::remap_test();
    // memory_set::load_user_apps_test();
}

pub fn translate_user_buffer(user_token: usize, buf: *const u8, len: usize) -> Vec<&'static [u8]> {
    let page_table = BorrowedPageTable::from_token(user_token);
    let buf_start = buf as usize;
    let buf_end = buf_start + len;
    let mut res = Vec::new();
    let mut start = buf_start;
    while start < buf_end {
        let va = VirtAddr(start);
        let mut vpn: VirtPageNum = va.floor();
        let bytes_array = unsafe { page_table.translate(vpn).unwrap().ppn().get_bytes_array() };
        vpn.step();
        let start_offset = va.page_offset();

        let page_end_va: VirtAddr = vpn.into();
        let end_va = page_end_va.min(VirtAddr::from(buf_end));
        if end_va.page_offset() == 0 {
            res.push(&bytes_array[start_offset..]);
        } else {
            res.push(&bytes_array[start_offset..end_va.page_offset()]);
        }
        start = end_va.into();
    }

    res
}
