use crate::trap::TrapContext;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct TaskContext {
    // return address
    ra: usize,
    // kernel stack address
    sp: usize,
    // callee-saved registers
    s: [usize; 12],
}

impl TaskContext {
    // when a task is first created, it will start from ``
    pub fn goto_restore_with_kernel_stack(sp: usize) -> Self {
        extern "C" {
            fn __restore(cx: *mut TrapContext);
        }

        Self {
            ra: __restore as usize,
            sp,
            s: [0; 12],
        }
    }
}
