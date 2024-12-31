//! The panic handler

use crate::sbi::shutdown;
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
    warn!("stack trace:");
    while fp != core::ptr::null() {
        let ra = unsafe { *fp.sub(1) };
        let next_fp = unsafe { *fp.sub(2) };
        warn!("fp: {:#x}, ra: {:#x}", fp as usize, ra);
        fp = next_fp as *const usize;
    }
}