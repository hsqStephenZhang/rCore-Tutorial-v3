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

mod stack_trace {
    use log::warn;

    pub const MAX_FRAME_DEPTH: usize = 32;

    pub struct StackTrace {
        _inner: (),
    }

    impl StackTrace {
        pub fn new() -> Self {
            warn!("stack trace:");
            Self { _inner: () }
        }
    }

    impl Drop for StackTrace {
        fn drop(&mut self) {
            warn!("stack trace done");
        }
    }
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
    let _stack_trace = stack_trace::StackTrace::new();

    let mut depth = 0;
    while is_valid(fp) && depth < stack_trace::MAX_FRAME_DEPTH {
        let ra = unsafe { *fp.sub(1) };
        let next_fp = unsafe { *fp.sub(2) };
        let ksym = crate::kallsyms::lookup(ra).unwrap_or("<unknown>");
        warn!("fp: {:#x}, ra: {:#x}, func: {}", fp as usize, ra, ksym);
        fp = next_fp as *const usize;
        depth += 1;
    }
}
