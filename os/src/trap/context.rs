use core::arch::asm;

use log::warn;
use riscv::register::{scause, sstatus, stval};

use crate::{
    syscall::syscall,
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::set_next_trigger,
};

#[repr(C)]
pub struct TrapContext {
    pub x: [usize; 32],
    pub sstatus: usize,
    pub sepc: usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }

    // create a trap context for a new app
    pub fn app_init_context(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::User);
        let mut context = Self {
            x: [0; 32],
            sstatus: sstatus.bits(),
            sepc: entry,
        };
        context.set_sp(sp);
        context
    }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    // TASK_MANAGER.add_user_time();
    let scause = scause::read();
    let stval = stval::read();

    match scause.cause() {
        // syscall
        scause::Trap::Exception(scause::Exception::UserEnvCall) => {
            cx.sepc += 4;
            cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
        }
        scause::Trap::Exception(scause::Exception::StoreFault)
        | scause::Trap::Exception(scause::Exception::StorePageFault) => {
            println!(
                "[kernel] StoreFault | StorePageFault at {:#x}, {:#x}",
                cx.sepc, stval
            );
            print_stack_trace();
            exit_current_and_run_next();
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            println!(
                "[kernel] IllegalInstruction at {:#x}, bad instruction {:#x}",
                cx.sepc, stval
            );
            print_stack_trace();
            exit_current_and_run_next();
        }
        scause::Trap::Interrupt(scause::Interrupt::SupervisorTimer) => {
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
    // TASK_MANAGER.add_kernel_time();

    cx
}

#[no_mangle]
pub fn print_stack_trace() {
    let mut fp: *const usize;
    unsafe {
        asm!("mv {}, fp", out(reg) fp);
    }
    warn!("stack trace:");
    while fp != core::ptr::null() {
        let ra = unsafe { *fp.sub(1) };
        let next_fp = unsafe { *fp.sub(2) };
        warn!("fp: {:#x}, ra: {:#x}", fp as usize, ra);
        fp = next_fp as *const usize;
    }
}
