pub mod fs;
pub mod process;

use crate::{
    config::*,
    loader::{get_app_base, get_app_stack_info},
};
use fs::sys_write;
use log::error;
use process::{sys_exit, sys_get_task_info, sys_yield, TaskInfo};

pub const SYSCALL_WRITE: usize = 64;
pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_YIELD: usize = 124;
pub const SYSCALL_GET_TASK_INFO: usize = 178;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TASK_INFO => sys_get_task_info(args[0] as *mut TaskInfo),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

pub fn user_buf_range_check(buf: *const u8, len: usize) -> bool {
    let addr = buf as usize;
    let current_task = crate::task::current_task();
    let (_, user_stack_top) = get_app_stack_info(current_task);
    let user_stack_bottom = user_stack_top - 4096;
    // TODO: leverage app's size to limit the range precisely
    let app_image_start = get_app_base(current_task);
    let app_image_end = app_image_start + APP_SIZE_LIMIT;
    // check the start and end of string
    // the [addr, addr + len) should be a subset of both stack and .data section
    let res = ((addr >= user_stack_bottom) && (addr + len < user_stack_top))
        || ((addr >= app_image_start) && (addr + len < app_image_end));
    if !res {
        error!("ERROR addr: {:x}, len: {}, user_stack_top: {:x}, user_stack_bottom: {:x}, app_image_start: {:x}, app_image_end: {:x}", addr, len, user_stack_top, user_stack_bottom, app_image_start, app_image_end);
    }
    res
}
