use core::cell::RefCell;

/// uni-processor unsafe cell (only safe for uni-processor singleton value)
pub struct UPUnsafeCell<T> {
    value: RefCell<T>,
}

unsafe impl<T> Sync for UPUnsafeCell<T> {}

impl<T> UPUnsafeCell<T> {
    /// # Safety: user has to ensure that the struct is used in uni-processor
    pub unsafe fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
        }
    }
    /// exclusive borrow
    pub fn borrow_mut(&self) -> core::cell::RefMut<T> {
        self.value.borrow_mut()
    }
}
