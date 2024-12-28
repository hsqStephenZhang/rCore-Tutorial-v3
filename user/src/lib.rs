#![no_std]
#![feature(panic_info_message)]
// #![feature(panic_internals)]
#![feature(linkage)]

#[macro_use]
pub mod console;
pub mod lang_term;
mod syscall;

pub use syscall::*;

pub const STDOUT: usize = 1;

pub fn write(fd: usize, buffer: &[u8]) -> isize {
    sys_write(fd, buffer)
}

pub fn exit(code: i32) -> isize {
    sys_exit(code)
}

pub fn get_task_info(task_info: &mut TaskInfo) -> isize {
    sys_get_task_info(task_info)
}

#[no_mangle]
pub fn debug_task_info() {
    let mut task_info = TaskInfo::default();
    println!("task info addr: {:p}", &task_info);
    get_task_info(&mut task_info);
    let name = &task_info.app_name[0..task_info.app_name_len];
    println!(
        "Task idx: {}, Task Name len: {} Task Name: {}",
        task_info.index,
        task_info.app_name_len,
        core::str::from_utf8(name).unwrap()
    );
}

#[no_mangle]
pub fn bad_task_info_user_arg() {
    const STACK_SIZE: usize = 0x1000;

    unsafe fn r_sp() -> usize {
        let mut sp: usize;
        core::arch::asm!("mv {}, sp", out(reg) sp);
        sp
    }

    unsafe fn stack_range() -> (usize, usize) {
        let sp = r_sp();
        let top = (sp + STACK_SIZE - 1) & (!(STACK_SIZE - 1));
        (top - STACK_SIZE, top)
    }

    let (bottom, top) = unsafe { stack_range() };
    let addr1 = 0x0 as *mut TaskInfo;
    let addr2 = (top - 5) as *mut TaskInfo;
    let addr3 = (bottom - 5) as *mut TaskInfo;
    let info1 = unsafe { core::mem::transmute::<*mut TaskInfo, &mut TaskInfo>(addr1) };
    assert_eq!(-1, get_task_info(info1));
    let info2 = unsafe { core::mem::transmute::<*mut TaskInfo, &mut TaskInfo>(addr2) };
    assert_eq!(-1, get_task_info(info2));
    let info3 = unsafe { core::mem::transmute::<*mut TaskInfo, &mut TaskInfo>(addr3) };
    assert_eq!(-1, get_task_info(info3));
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
