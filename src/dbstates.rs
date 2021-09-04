use std::sync::atomic::{AtomicBool, Ordering};

static SHUTTING_DOWN:AtomicBool = AtomicBool::new(false);

pub fn is_shutting_down() -> bool {
	SHUTTING_DOWN.load(Ordering::Relaxed)
}

pub fn start_shutting_down() -> bool {
	match SHUTTING_DOWN.compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst) {
		Ok(_) => true,
		Err(_) => false
	}
}