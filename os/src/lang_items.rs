//! The panic handler

use crate::{
    mm::{memory_set::KERNEL_SPACE, VirtAddr},
    sbi::shutdown,
};
use core::panic::PanicInfo;
use log::*;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!("[kernel] Panicked: {}", info.message().unwrap());
    }
    print_stack_trace();
    shutdown(true)
}

#[no_mangle]
pub fn print_stack_trace() {
    let mut fp: *const usize;
    unsafe {
        core::arch::asm!("mv {}, fp", out(reg) fp);
    }
    
    fn is_valid(fp: *const usize) -> bool {
        if fp == core::ptr::null() {
            return false;
        }
        let ra_addr = unsafe { fp.sub(1) as usize };
        let ra_vpn = VirtAddr::from(ra_addr).into();
        let next_fp_addr = unsafe { fp.sub(2) as usize };
        let next_fp_vpn = VirtAddr::from(next_fp_addr).into();
        let kernel_space = KERNEL_SPACE.exclusive_access();
        if kernel_space.translate(ra_vpn).is_none() || kernel_space.translate(next_fp_vpn).is_none()
        {
            return false;
        }
        true
    }
    warn!("stack trace:");
    while is_valid(fp) {
        let ra = unsafe { *fp.sub(1) };
        let next_fp = unsafe { *fp.sub(2) };
        warn!("fp: {:#x}, ra: {:#x}", fp as usize, ra);
        fp = next_fp as *const usize;
    }
    warn!("stack trace done");
}
