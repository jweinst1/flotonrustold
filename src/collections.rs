use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use crate::shared::*;

struct HashScheme( /*Base */u64, /*Adder*/ u32, /*Incrementor*/ u32);

impl HashScheme {
	fn hash(&self, data:&[u8]) -> u64 {
		let mut base = self.0;
		for b in data.iter() {
			base = (base << 5) + base + b;
		}
		base
	}
}