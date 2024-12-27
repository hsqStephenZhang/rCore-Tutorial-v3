use crate::batch::run_next_app;

pub fn sys_exit(code: i32) -> isize {
    println!("exit: {}", code);
    run_next_app();
}