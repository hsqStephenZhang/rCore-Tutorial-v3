// initcall.rs

use log::info;

// Type for initialization functions
pub type InitCallFn = unsafe fn() -> i32;

// Function to run all initcalls in order
#[no_mangle]
pub unsafe fn do_initcalls() {
    extern "C" {
        fn sinitdata();
        fn einitdata();
    }

    let initcalls = sinitdata as *const InitCallFn;
    let num_initcalls =
        (einitdata as usize - sinitdata as usize) / core::mem::size_of::<InitCallFn>();
    let initcalls = core::slice::from_raw_parts(initcalls, num_initcalls);
    info!("do_initcalls, num: {}", num_initcalls);
    for initcall in initcalls {
        (*initcall)();
    }
    info!("do_initcalls done");
}

// Example usage, we may
#[link_section = ".init.text"]
#[no_mangle]
unsafe fn example_init() -> i32 {
    println!("Hello from example0_init!");
    0
}

#[link_section = ".initcall.init"]
#[no_mangle]
#[used]
static EXAMPLE_INITCALL: crate::initcall::InitCallFn = example_init;

#[link_section = ".init.text"]
#[no_mangle]
unsafe fn example1_init() -> i32 {
    println!("Hello from example1_init!");
    0
}

#[link_section = ".initcall1.init"]
#[no_mangle]
#[used]
static EXAMPLE1_INITCALL: crate::initcall::InitCallFn = example_init;

#[link_section = ".init.text"]
#[no_mangle]
unsafe fn example2_init() -> i32 {
    println!("Hello from example2_init!");
    0
}

#[link_section = ".initcall2.init"]
#[no_mangle]
#[used]
static EXAMPLE2_INITCALL: crate::initcall::InitCallFn = example2_init;
