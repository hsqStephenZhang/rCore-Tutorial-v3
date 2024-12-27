use riscv::register::{scause, sstatus, stval};

use crate::{batch::run_next_app, syscall::syscall};

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
            run_next_app();
        }
        scause::Trap::Exception(scause::Exception::IllegalInstruction) => {
            println!(
                "[kernel] IllegalInstruction at {:#x}, bad instruction {:#x}",
                cx.sepc, stval
            );
            run_next_app();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }

    cx
}
