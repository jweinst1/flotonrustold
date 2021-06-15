use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::{thread, time, ptr};

struct SharedCount<T>(*mut T, AtomicUsize);

impl<T> SharedCount<T> {
    fn make(val:T) -> *mut SharedCount<T> {
        Box::into_raw(Box::new(SharedCount(Box::into_raw(Box::new(val)), AtomicUsize::new(1))))
    }
}

struct AccessCounted<T> {
	ptr:AtomicPtr<SharedCount<T>>,
	clone_access:AtomicUsize,
	reset_access:AtomicUsize,
	missed:AtomicUsize
};