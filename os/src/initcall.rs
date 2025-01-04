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
export_func_simple!(do_initcalls);

// Example usage, we may
#[init_section(level = 1)]
unsafe fn example_init() -> i32 {
    println!("Hello from example0_init!");
    0
}

#[init_section(level = 1)]
unsafe fn example1_init() -> i32 {
    println!("Hello from example1_init!");
    0
}

#[init_section(level = 2)]
unsafe fn example2_init() -> i32 {
    println!("Hello from example2_init!");
    0
}

#[init_section(level = 3)]
unsafe fn example3_init() -> i32 {
    println!("Hello from example3_init!");
    0
}
