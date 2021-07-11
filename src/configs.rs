use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::ptr;

pub struct Configs {
	thread_count:AtomicU32,
	free_list_limit:AtomicU32
}

static CONFIGS_INST:AtomicPtr<Configs> = AtomicPtr::new(ptr::null_mut());

impl Configs {
	// can be calld multiple times
	pub fn initialize() {
		let config_obj = Configs{thread_count:AtomicU32::new(5), free_list_limit:AtomicU32::new(2)};
		let swapped_out = CONFIGS_INST.swap(Box::into_raw(Box::new(config_obj)), Ordering::SeqCst);
		if swapped_out != ptr::null_mut() {
			unsafe { drop(Box::from_raw(swapped_out)); }
		}
	}

	pub fn instance() -> *const Configs {
		CONFIGS_INST.load(Ordering::SeqCst)
	}

	pub fn get_free_list_limit() -> u32 {
		unsafe {
			match CONFIGS_INST.load(Ordering::SeqCst).as_ref() {
				Some(r) => r.free_list_limit.load(Ordering::SeqCst),
				None => panic!("Attempted to read free_list_limit, but configs was null")
			}
		}
	}

	pub fn get_thread_count() -> u32 {
		unsafe {
			match CONFIGS_INST.load(Ordering::SeqCst).as_ref() {
				Some(r) => r.thread_count.load(Ordering::SeqCst),
				None => panic!("Attempted to read thread_count, but configs was null")
			}
		}
	}
}