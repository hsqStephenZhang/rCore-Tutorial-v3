//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].

mod context;

use crate::config::{TRAMPOLINE, TRAP_CONTEXT};
use crate::syscall::syscall;
use crate::task::{
    current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec,
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_handler();
}

/// timer interrupt enabled
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

pub fn set_user_trap_handler() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn panic_kernel() -> ! {
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    println!("[kernel] TRAP: {:?}, stval = {:#x}", scause.cause(), stval);
    panic!("[kernel] UNEXPECTED TRAP FROM KERNEL!");
}
export_func_simple!(panic_kernel);

fn set_kernel_trap_handler() {
    unsafe {
        stvec::write(panic_kernel as _, TrapMode::Direct);
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_handler();
    let mut cx = current_trap_cx();
    let scause = scause::read(); // get trap cause
    let stval = stval::read(); // get extra value
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;
            let ret = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            cx = current_trap_cx();
            cx.x[10] = ret;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::LoadPageFault) | Trap::Exception(Exception::LoadFault) => {
            println!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            exit_current_and_run_next(-2);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    trap_return();
}
export_func_simple!(trap_handler);

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_handler();

    extern "C" {
        fn __alltraps();
        fn __restore();
    }

    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();

    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}
export_func_simple!(trap_return);

pub use context::TrapContext;
