//! File and filesystem-related syscalls

use crate::{mm::translate_user_buffer_mut, sbi::console_getchar, task::{current_user_token, suspend_current_and_run_next}};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
#[no_mangle]
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = translate_user_buffer_mut(current_user_token(), buf, len);
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

#[no_mangle]
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translate_user_buffer_mut(current_user_token(), buf, len);
            unsafe { buffers[0].as_mut_ptr().write_volatile(ch); }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}