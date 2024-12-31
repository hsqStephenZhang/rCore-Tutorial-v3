#![no_std]
#![no_main]

use core::ptr::slice_from_raw_parts_mut;

use user_lib::{config::PAGE_SIZE, mmap};

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("\nmmap APP running...\n");
    let start = 0x1000000;
    let res = mmap(start, 4096, 0b11);
    assert_eq!(res, 0);

    let new_page = unsafe {
        &mut *slice_from_raw_parts_mut(start as usize as *const u8 as *mut u8, PAGE_SIZE)
    };
    for pos in 0..PAGE_SIZE {
        new_page[pos] = 1;
    }

    let content = "hello".as_bytes();
    new_page[0..content.len()].copy_from_slice(content);
    println!("write ok");
    println!(
        "read content and print: {}",
        core::str::from_utf8(&new_page[0..content.len()]).unwrap()
    );
    println!("mmap APP finished.\n");

    0
}
