//! memory management

pub mod address;
pub mod frame_allocator;
pub mod heap_allocator;
pub mod memory_set;
pub mod page_table;

pub use address::*;
use alloc::{string::String, vec::Vec};
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

pub fn translate_user_buffer_mut(
    user_token: usize,
    buf: *const u8,
    len: usize,
) -> Vec<&'static mut [u8]> {
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
            res.push(&mut bytes_array[start_offset..]);
        } else {
            res.push(&mut bytes_array[start_offset..end_va.page_offset()]);
        }
        start = end_va.into();
    }

    res
}

pub fn translate_user_str(user_token: usize, buf: *const u8) -> String {
    let page_table = BorrowedPageTable::from_token(user_token);
    let mut res = Vec::new();
    let mut start = buf as usize;
    loop {
        let va = VirtAddr(start);
        let mut vpn: VirtPageNum = va.floor();
        let bytes_array = unsafe { page_table.translate(vpn).unwrap().ppn().get_bytes_array() };
        let start_offset = va.page_offset();

        vpn.step();
        let end_va: VirtAddr = vpn.into();

        let array = if end_va.page_offset() == 0 {
            &mut bytes_array[start_offset..]
        } else {
            &mut bytes_array[start_offset..end_va.page_offset()]
        };
        // find '\0' in the array
        match array.iter().position(|&x| x == 0) {
            Some(pos) => {
                res.extend_from_slice(&array[..pos]);
                break;
            }
            None => res.extend_from_slice(array),
        }

        start = end_va.into();
    }

    String::from_utf8(res).unwrap()
}

pub fn copy_to_user<T: Sized>(user_token: usize, src: &T, dst: *mut T) {
    let mut user_bufs =
        translate_user_buffer_mut(user_token, dst as *const u8, core::mem::size_of::<T>());
    assert_eq!(
        user_bufs.iter().map(|buf| buf.len()).sum::<usize>(),
        core::mem::size_of::<T>()
    );
    let src_buf = unsafe {
        core::slice::from_raw_parts(src as *const T as *const u8, core::mem::size_of::<T>())
    };
    let mut offset = 0;
    for buf in user_bufs.iter_mut() {
        buf.copy_from_slice(&src_buf[offset..offset + buf.len()]);
        offset += buf.len();
    }
}
