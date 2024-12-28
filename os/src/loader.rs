use core::sync::atomic;

use crate::{config::*, trap::TrapContext};

static NUM_APP: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
static mut APP_STARTS: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];

/// SAFETY: only call this function after `load_apps`
pub unsafe fn get_app_num() -> usize {
    NUM_APP.load(atomic::Ordering::Relaxed)
}

pub fn get_app_base(idx: usize) -> usize {
    APP_BASE_ADDRESS + idx * APP_SIZE_LIMIT
}

pub unsafe fn get_app_range(idx: usize) -> (usize, usize) {
    (
        get_app_base(idx),
        get_app_base(idx) + APP_STARTS[idx + 1] - APP_STARTS[idx],
    )
}

pub unsafe fn load_apps() {
    extern "C" {
        fn _num_app();
        fn _app_names();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = num_app_ptr.read_volatile();
    NUM_APP.store(num_app, atomic::Ordering::Relaxed);

    // why not read_volative ?
    let app_start_raw = core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
    APP_STARTS[0..=num_app].copy_from_slice(app_start_raw);

    for idx in 0..num_app {
        // 1. clear dst
        let start = get_app_base(idx);
        let end = start + APP_SIZE_LIMIT;
        (start..end).for_each(|x| unsafe { (x as *mut u8).write_volatile(0) });

        // 2. construct src
        let app_src = core::slice::from_raw_parts(
            APP_STARTS[idx] as *const u8,
            APP_STARTS[idx + 1] - APP_STARTS[idx],
        );

        // 3. copy to dst
        let app_dst = core::slice::from_raw_parts_mut(start as *mut _, app_src.len());
        app_dst.copy_from_slice(app_src);
    }

    for i in 0..num_app {
        let (start, end) = get_app_range(i);
        println!("app[{}] loaded at [{:#x}, {:#x})", i, start, end);
    }

    // TODO: read app names
    // let mut app_names = [([0; 16], 0); MAX_APP_NUM];

    // read app names
    // let mut current = _app_names as *const u8;
    // for i in 0..num_app {
    //     let mut len = 0;
    //     while *current.add(len) != 0 {
    //         len += 1;
    //     }
    //     let name = core::slice::from_raw_parts(current, len);
    //     assert!(name.len() < 16);
    //     app_names[i].0[0..len].copy_from_slice(name);
    //     app_names[i].1 = len;
    //     current = current.add(len + 1);
    // }
}

/// layout of kernel stack
/// | TrapContext | Trap Handler Function Stack Frame ...
#[repr(align(4096))]
#[derive(Clone, Copy)]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACKS: [KernelStack; MAX_APP_NUM] = [KernelStack {
    data: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACKS: [UserStack; MAX_APP_NUM] = [UserStack {
    data: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

impl KernelStack {
    fn top(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    /// push a TrapContext to the top of the stack
    pub fn push_context(&self, trap_cx: TrapContext) -> usize {
        let trap_cx_ptr = (self.top() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *trap_cx_ptr = trap_cx;
        }
        trap_cx_ptr as usize
    }
}

impl UserStack {
    fn top(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

/// create trap context(in kernel stack) for a new app
/// return the start of the kernel stack that can be used as trap handler function stack frame
pub fn init_app_ctx(idx: usize) -> usize {
    KERNEL_STACKS[idx].push_context(TrapContext::app_init_context(
        get_app_base(idx),
        USER_STACKS[idx].top(),
    ))
}

pub fn get_app_stack_info(idx: usize) -> (usize, usize) {
    (KERNEL_STACKS[idx].top(), USER_STACKS[idx].top())
}

// safety: only call this function after `load_apps`
pub fn print_stack_infos(num: usize) {
    for i in 0..num {
        let (kernel_stack_top, user_stack_top) = get_app_stack_info(i);
        println!(
            "app[{}] kernel stack top: {:#x}, user stack top: {:#x}",
            i, kernel_stack_top, user_stack_top
        );
    }
}
