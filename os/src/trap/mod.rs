mod context;

use core::arch::global_asm;

pub use context::{print_stack_trace, TrapContext};
use riscv::register::stvec;

global_asm!(include_str!("trap.S"));

pub fn init() {
    extern "C" {
        fn __alltraps();
    }
    unsafe {
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
}
