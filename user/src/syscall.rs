use core::arch::asm;

// the args are all usize, the return type is isize
pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        )
    }
    ret
}

pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_WRITE: usize = 64;
pub const SYSCALL_GET_TASK_INFO: usize = 178;

pub fn sys_exit(code: i32) -> isize {
    syscall(SYSCALL_EXIT, [code as usize, 0, 0])
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub const APP_NAME_MAX_LEN: usize = 16;

#[repr(C)]
#[derive(Default, Debug)]
pub struct TaskInfo {
    pub index: usize,
    pub app_name: [u8; APP_NAME_MAX_LEN],
    pub app_name_len: usize,
}

pub fn sys_get_task_info(task_info: &mut TaskInfo) -> isize {
    syscall(
        SYSCALL_GET_TASK_INFO,
        [task_info as *const _ as usize, 0, 0],
    )
}
