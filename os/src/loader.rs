//! Loading user applications into memory
//!
//! For chapter 3, user applications are simply part of the data included in the
//! kernel binary, so we only need to copy them to the space allocated for each
//! app to load them. We also allocate fixed spaces for each task's
//! [`KernelStack`] and [`UserStack`].

use alloc::vec::Vec;
use log::info;

extern "C" {
    fn _num_app();
    fn _app_names();
}

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get physical data of nth user app
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    let num_app = get_num_app();
    assert!(app_id < num_app);
    let num_app_ptr = _num_app as usize as *const usize;

    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static::lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = unsafe { load_app_names() };
}

unsafe fn load_app_names() -> Vec<&'static str> {
    let num_app = get_num_app();
    let mut names = Vec::new();
    let app_name_ptr_start = _app_names as usize as *const u8;
    let mut offset = 0;
    for _ in 0..num_app {
        let current_name_start = offset;
        while *app_name_ptr_start.add(offset) != 0 {
            offset += 1;
        }
        let len = offset - current_name_start;
        let name = core::str::from_utf8(
            core::slice::from_raw_parts(app_name_ptr_start.add(current_name_start), len)
        ).unwrap();
        names.push(name);
        offset += 1;
    }
    names
}

pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    APP_NAMES
        .iter()
        .position(|&x| x == name)
        .map(|i| get_app_data(i))
}

pub fn print_task_names() {
    for name in APP_NAMES.iter() {
        info!("{}", name);
    }

    assert!(get_app_data_by_name("user_shell").is_some());
    assert!(get_app_data_by_name("initproc").is_some());
    assert!(get_app_data_by_name("initproc2").is_none());
}
