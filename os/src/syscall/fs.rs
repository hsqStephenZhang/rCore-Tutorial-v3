use super::user_buf_range_check;

const FD_STDOUT: usize = 1;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            if user_buf_range_check(buf, len) {
                let buffer = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(buffer).unwrap();
                print!("{}", str);
                len as isize
            } else {
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
