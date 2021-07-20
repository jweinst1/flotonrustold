use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use std::time::{Instant};
use std::convert::TryFrom;
use std::mem::{self, MaybeUninit};
use crate::shared;
use crate:containers::Container;

static DEFAULT_HASH_BASE:u64 = 0x5331;

#[derive(Debug)]
struct HashScheme( /*Base */u64);

impl HashScheme {
	fn hash(&self, data:&[u8]) -> u64 {
		let mut base = self.0;
		for b in data.iter() {
			base = ((base << (*b & 0x2a)) | (base >> (*b & 0x2a))) ^ (*b as u64);
		}
		base
	}

	fn evolve(&self) -> HashScheme {
		let tick = shared::check_time();
		HashScheme(self.0 ^ tick)
	}
}

#[derive(Debug)]
enum BitTrie<T> {
	Connect([AtomicPtr<BitTrie<T>>; 2]),
	Entry(HashScheme, [AtomicPtr<BitTrie<T>>; 2]),
	Item(String, Shared<Container<T>>, /*Entry of next layer*/ AtomicPtr<BitTrie<T>>)
}

impl<T> BitTrie<T> {
	fn new_item(key:String, val:Container<T>) -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Item(key, 
			                                     shared::Shared::new_val(val), 
			                                     AtomicPtr::new(ptr::null_mut()))))
	}

	fn new_connect() -> *mut BitTrie<T> {
		Box::into_raw(Box::new(
			             BitTrie::Connect([ 
			             	          AtomicPtr::new(ptr::null_mut()), 
			             	          AtomicPtr::new(ptr::null_mut())
			             	                  ])
			             ))
	}

	fn new_entry() -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Entry(
			                 HashScheme(DEFAULT_HASH_BASE), [ 
			             	          AtomicPtr::new(ptr::null_mut()), 
			             	          AtomicPtr::new(ptr::null_mut())
			             	                  ]
			             	          )
		))
	}

	fn insert(trie: *mut BitTrie<T>, key:String, val:T) {

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