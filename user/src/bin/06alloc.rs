#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[macro_use]
extern crate alloc;

use buddy_system_allocator::LockedHeap;
use user_lib::{config::PAGE_SIZE, sbrk};

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

fn init_heap() -> isize {
    let origin_brk = sbrk((PAGE_SIZE * 8) as i32);

    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(origin_brk as usize, PAGE_SIZE * 8);
    }
    0
}

#[no_mangle]
fn main() -> i32 {
    println!("Test alloc start.");
    init_heap();

    let v = vec![1, 2, 3, 4, 5];
    assert!(v.len() == 5);

    for i in 0..5 {
        println!("v[{}] = {}", i, v[i]);
    }

    println!("Test alloc OK!");
    0
}
