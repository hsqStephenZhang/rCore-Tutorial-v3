//! File and filesystem-related syscalls

use crate::{mm::translate_user_buffer, task::current_user_token};

const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
#[no_mangle]
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = translate_user_buffer(current_user_token(), buf, len);
            // TODO: it might not be ok to transmute a truncated slice to a str
            // safe only for ASCII
            for s in slice {
                print!("{}", core::str::from_utf8(s).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}
