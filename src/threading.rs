use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::{thread, ptr};
use crate::traits::NewType;

const THREAD_COUNT_DEFAULT:usize = 8;
static THREAD_COUNT:AtomicUsize = AtomicUsize::new(THREAD_COUNT_DEFAULT);

pub fn get_thread_count() -> usize {
    THREAD_COUNT.load(Ordering::SeqCst)
}

pub fn set_thread_count(count:usize) {
    THREAD_COUNT.store(count, Ordering::SeqCst);
}

struct UnitInfo {
	tid:usize,
	woken:AtomicBool
}

impl NewType for UnitInfo {
	fn new() -> Self {
		UnitInfo{tid:0, woken:AtomicBool::new(false)}
	}
}