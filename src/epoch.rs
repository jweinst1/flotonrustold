use std::time::Instant;
use std::mem::MaybeUninit;
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_timer_works() {
        set_epoch();
        let t1 = check_time();
        let t2 = check_time();
        if !(t1 < t2) {
            panic!("monotonic t1:{:?} is not less than t2:{:?}", t1, t2);
        }
    }
}