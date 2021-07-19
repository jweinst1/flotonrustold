use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use std::time::{Instant};
use std::convert::TryFrom;
use std::mem::{self, MaybeUninit};
use crate::shared;

static DEFAULT_HASH_BASE:u64 = 0x5331;

#[derive(Debug)]
struct HashScheme( /*Base */u64);

impl HashScheme {
	fn hash(&self, data:&[u8]) -> u64 {
		let mut base = self.0;
		for b in data.iter() {
			base = ((base << (*b & 0x2d)) | (base >> (*b & 0x2d))) ^ (*b as u64);
		}
		base
	}

	fn evolve(&self) -> HashScheme {
		let tick = shared::check_time();
		HashScheme(self.0 ^ tick)
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn evolve_hash_works() {
    	shared::set_epoch();
    	let hs = HashScheme(DEFAULT_HASH_BASE);
    	let s = String::from("Hello!");
    	let hash1 = hs.hash(s.as_bytes());
    	let hs2 = hs.evolve();
    	let hash2 = hs2.hash(s.as_bytes());
    	assert!(hash1 != hash2);
    }
}