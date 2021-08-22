use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::time::Instant;
use std::mem::MaybeUninit;
use std::convert::TryFrom;
use crate::traits::*;

// Functions as the per database wide set of configurables
// and runtime variables
#[derive(Debug)]
struct Settings {
	epoch:Instant
}

impl NewType for Settings {
	fn new() -> Self {
		Settings{epoch:Instant::now()}
	}
}

impl Settings {
	pub fn set_epoch(&mut self, epc:Instant) {
		self.epoch = epc;
	}

	pub fn get_time(&self) -> u64 {
		match u64::try_from(self.epoch.elapsed().as_nanos()) {
			Ok(v) => v,
			Err(e) => panic!("Could not convert monotonic tick to u64, err: {:?}", e)
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_works() {
    	let mut sgs = Settings::new();
    	sgs.set_epoch(Instant::now());
    	let t1 = sgs.get_time();
    	let t2 = sgs.get_time();
    	assert!(t2 > t1);
    }
}
