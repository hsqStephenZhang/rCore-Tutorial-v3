use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};

use super::APP_NAME_MAX_LEN;

pub fn sys_exit(code: i32) -> ! {
    println!("exit: {}", code);
    exit_current_and_run_next();
    panic!("unreachable after exit");
}

pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

#[repr(C)]
pub struct TaskInfo {
    pub index: usize,
    pub app_name: [u8; APP_NAME_MAX_LEN],
    pub app_name_len: usize,
}

#[no_mangle]
pub fn sys_get_task_info(_task_info: *mut TaskInfo) -> isize {
    // use crate::syscall::user_buf_range_check;
    // TDDO: check write permission of the user buffer
    // let start_addr = task_info as *mut u8;
    // let len = core::mem::size_of::<TaskInfo>();
    // if user_buf_range_check(start_addr, len) == false {
    //     return -1;
    // }

    // let app_manager = APP_MANAGER.borrow_mut();
    // let idx = app_manager.get_current_app();
    // let name = app_manager.get_run_app_name();
    // assert!(name.len() < APP_NAME_MAX_LEN);
    // let task_info = unsafe { &mut *task_info };
    // task_info.index = idx;
    // task_info.app_name_len = name.len();
    // task_info.app_name[..name.len()].copy_from_slice(name);
    0
}
