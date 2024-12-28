use log::error;

use crate::batch::{user_stack_top, APP_BASE_ADDRESS, APP_SIZE_LIMIT};

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let addr = buf as usize;
            let user_stack_top = user_stack_top();
            let user_stack_bottom = user_stack_top - 4096;
            let app_image_start = APP_BASE_ADDRESS;
            let app_image_end = APP_BASE_ADDRESS + APP_SIZE_LIMIT;
            // check the start and end of string
            // the [addr, addr + len) should be a subset of both stack and .data section
            if ((addr >= user_stack_bottom) && (addr + len < user_stack_top))
                || ((addr >= app_image_start) && (addr + len < app_image_end))
            {
                let buffer = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(buffer).unwrap();
                print!("{}", str);
                len as isize
            } else {
                error!("ERROR addr: {:x}, len: {}, user_stack_top: {:x}, user_stack_bottom: {:x}, app_image_start: {:x}, app_image_end: {:x}", addr, len, user_stack_top, user_stack_bottom, app_image_start, app_image_end);
                -1
            }
        }
        // return -1 if fd is not supported
        // DONT PANIC for passing test case
        _ => {
            println!(
                "sys_write: not support fd: {}, buf: {}, len: {}",
                fd, buf as usize, len
            );
            // panic!("sys_write: not support fd: {}", fd);
            -1
        }
    }
}
