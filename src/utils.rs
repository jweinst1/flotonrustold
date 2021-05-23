
pub mod ptrs {
	use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, Ordering};
	use std::thread;
	use std::ptr;

	pub struct SharedPtr<T> {
	    pub ptr:AtomicPtr<T>,
	    pub counter:AtomicPtr<AtomicUsize>
	}

	impl<T> Default for SharedPtr<T> {
	    fn default() -> Self { 
	       SharedPtr{ptr:AtomicPtr::new(ptr::null_mut()),
	                 counter:AtomicPtr::new(ptr::null_mut())}
	    }
	}
}