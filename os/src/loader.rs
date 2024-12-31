//! Loading user applications into memory
//!
//! For chapter 3, user applications are simply part of the data included in the
//! kernel binary, so we only need to copy them to the space allocated for each
//! app to load them. We also allocate fixed spaces for each task's
//! [`KernelStack`] and [`UserStack`].

extern "C" {
    fn _num_app();
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
