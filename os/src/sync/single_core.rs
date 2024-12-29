use core::cell::RefCell;

/// uni-processor unsafe cell (only safe for uni-processor singleton value)
pub struct UPSafeCell<T> {
    value: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// # Safety: user has to ensure that the struct is used in uni-processor
    pub unsafe fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
        }
    }
    /// exclusive borrow
    pub fn exclusive_access(&self) -> core::cell::RefMut<T> {
        self.value.borrow_mut()
    }
}
