#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    user_lib::debug_task_info();
    println!("Hello, world!");
    0
}
