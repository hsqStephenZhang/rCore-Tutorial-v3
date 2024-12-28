use core::arch::asm;

use crate::{sbi::shutdown, sync::UPUnsafeCell, trap::TrapContext};
use lazy_static::lazy_static;

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl UserStack {
    fn top(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

impl KernelStack {
    fn top(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&self, ctx: TrapContext) -> &'static mut TrapContext {
        let ctx_ptr = (self.top() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *ctx_ptr = ctx;
        }
        unsafe { &mut *ctx_ptr }
    }
}

lazy_static! {
    pub static ref APP_MANAGER: UPUnsafeCell<AppManager> = unsafe {
        UPUnsafeCell::new({
            extern "C" {
                fn _num_app();
            }

            let num_app_ptr = _num_app as usize as *const usize;
            let num_app = num_app_ptr.read_volatile();
            let mut app_start = [0; MAX_APP_NUM + 1];
            // why not read_volative ?
            let app_start_raw = core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
            app_start[0..=num_app].copy_from_slice(app_start_raw);

            AppManager {
                num_app,
                current_app: 0,
                app_start: app_start,
            }
        })
    };
}

pub struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    // single app layout
    // 8 byte start address + 8 byte end adress + actual binary data
    pub unsafe fn load_app(&self, idx: usize) {
        if idx > self.num_app {
            println!("all app has been loaded, shutdown");
            shutdown(false);
        }
        println!("loading app_{}", idx);

        // 1. clear dst
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut _, APP_SIZE_LIMIT).fill(0);

        // 2. read app
        let start = self.app_start[idx];
        let end = self.app_start[idx + 1];
        let app_src = core::slice::from_raw_parts(start as *const u8, end - start);

        // 3. copy to dst
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);

        asm!("fence.i");
    }

    fn print_app_info(&self) {
        for i in 0..self.num_app {
            println!(
                "app_{}: {:#x} - {:#x}",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    fn get_current_app(&self) -> usize {
        self.current_app
    }

    // move to next app (without loop)
    fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

pub fn init() {
    print_app_info();
}

pub fn print_app_info() {
    APP_MANAGER.borrow_mut().print_app_info();
}

pub fn user_stack_top() -> usize {
    USER_STACK.top()
}

/// load next app and move the cursor to the app after it,
pub fn run_next_app() -> ! {
    let mut app_manager = APP_MANAGER.borrow_mut();
    let current_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);

    extern "C" {
        fn __restore(cx: *mut TrapContext);
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(TrapContext::app_init_context(
            APP_BASE_ADDRESS,
            USER_STACK.top(),
        )) as *mut _);
    }

    panic!("")
}

pub fn stack_info() -> (usize, usize) {
    (KERNEL_STACK.top(), USER_STACK.top())
}
