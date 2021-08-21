use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::time::Instant;
use std::mem::MaybeUninit;
use std::convert::TryFrom;
use crate::traits::*;

// Functions as the per database wide set of configurables
// and runtime variables
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
}

