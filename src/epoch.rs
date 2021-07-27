use std::time::{Duration, Instant};
use std::mem::{self, MaybeUninit};
use std::convert::TryFrom;

// Note, must be initialized from single threaded context
static mut MONOTONIC_EPOCH:MaybeUninit<Instant> = MaybeUninit::<Instant>::uninit();

pub fn set_epoch() {
    unsafe {
        MONOTONIC_EPOCH.as_mut_ptr().write(Instant::now());
    }
}

pub fn check_time() -> u64 {
	unsafe {
        // todo, make precision configurable
		match u64::try_from(MONOTONIC_EPOCH.assume_init().elapsed().as_nanos()) {
			Ok(v) => v,
			Err(e) => panic!("Could not convert monotonic tick to u64, err: {:?}", e)
		}
	}
}