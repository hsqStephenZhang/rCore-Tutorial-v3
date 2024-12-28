use crate::batch::{run_next_app, APP_MANAGER, APP_NAME_MAX_LEN};

pub fn sys_exit(code: i32) -> isize {
    println!("exit: {}", code);
    run_next_app();
}

#[repr(C)]
pub struct TaskInfo {
    pub index: usize,
    pub app_name: [u8; APP_NAME_MAX_LEN],
    pub app_name_len: usize,
}

#[no_mangle]
pub fn sys_get_task_info(task_info: *mut TaskInfo) -> isize {
    // TDDO: check validity of the argument
    let app_manager = APP_MANAGER.borrow_mut();
    let idx = app_manager.get_current_app();
    let name = app_manager.get_run_app_name();
    assert!(name.len() < APP_NAME_MAX_LEN);
    let task_info = unsafe { &mut *task_info };
    task_info.index = idx;
    task_info.app_name_len = name.len();
    task_info.app_name[..name.len()].copy_from_slice(name);
    0
}
