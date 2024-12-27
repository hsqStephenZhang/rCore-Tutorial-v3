#![no_std]
#![feature(panic_info_message)]
// #![feature(panic_internals)]
#![feature(linkage)]

#[macro_use]
pub mod console;
pub mod lang_term;
mod syscall;

use syscall::*;

pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}

pub fn exit(code: i32) -> isize {
    sys_exit(code)
}

#[no_mangle]
#[linkage = "weak"]
fn main() -> i32 {
    panic!("weak linkage func, should not appear here")
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("should not reach here after exit");
}

fn clear_bss() {
    // defined in linker.ld
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}
