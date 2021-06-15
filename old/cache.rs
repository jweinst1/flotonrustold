use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
use std::ptr;
use crate::bit_trie::BitTrie;

pub struct Cache<T> {
	key_size:usize,
	items:BitTrie<T>,
	head:AtomicI64
}

impl<T> Cache<T> {
	pub fn new(key_size:usize) -> Cache<T> {
		Cache{key_size:key_size, items:BitTrie::new(), head:AtomicI64::new(0)}
	}

	pub fn push(&self, ptr: *mut T, attempts:usize) -> bool {
		for _ in 0..attempts {
			let place = self.head.fetch_add(1, Ordering::SeqCst);
			if place < 0 {
				continue;
			}
			if self.items.switch(place as u64, self.key_size, ptr) {
				return true;
			}
		}
		return false;
	}

	pub fn pop(&self, attempts:usize) -> Option<*mut T> {
		for _ in 0..attempts {
			let place = self.head.fetch_sub(1, Ordering::SeqCst) - 1;
			if place < 0 {
				// Nothing to pop
				self.head.fetch_add(1, Ordering::SeqCst);
				return None;
			}
			match self.items.flip(place as u64, self.key_size) {
				Some(p) => { return Some(p); },
				None => ()
			}
		}
		return None;
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};

    #[test]
    fn push_pop_works() {
    	let cache = Cache::<i32>::new(16);
    	let ptr = Box::into_raw(Box::new(5));
    	assert!(cache.push(ptr, 1));
    	match cache.pop(1) {
    		Some(p) => unsafe {
    			assert!(*p.as_ref().unwrap() == 5);
    			drop(Box::from_raw(p));
    		},
    		None => { panic!("Expected pointer from pop() but got None"); }
    	}
    }

    #[test]
    fn empty_pop_fails() {
    	let cache = Cache::<i32>::new(16);
    	for _ in 0..3 {
    		let ptr = Box::into_raw(Box::new(5));
    		assert!(cache.push(ptr, 1));
    	}

    	for _ in 0..3 {
	    	match cache.pop(1) {
	    		Some(p) => unsafe {
	    			assert!(*p.as_ref().unwrap() == 5);
	    			drop(Box::from_raw(p));
	    		},
	    		None => { panic!("Expected pointer from pop() but got None"); }
	    	}
    	}
    	// empty pop
    	match cache.pop(1) {
    		Some(unexpected) => { panic!("Expected none from empty pop() but got {:?}", unexpected); },
    		None => { assert!(cache.head.load(Ordering::SeqCst) == 0); }
    	}
    }

    #[test]
    fn failed_push_progresses() {
    	let cache = Cache::<i32>::new(16);
    	cache.items.switch(0, cache.key_size, Box::into_raw(Box::new(5)));
    	// 2 attempts should work here
    	assert!(cache.push(Box::into_raw(Box::new(5)), 2));
    	for _ in 0..2 {
	    	match cache.pop(1) {
	    		Some(p) => unsafe {
	    			assert!(*p.as_ref().unwrap() == 5);
	    			drop(Box::from_raw(p));
	    		},
	    		None => { panic!("Expected pointer from pop() but got None"); }
	    	}
    	}
    	assert!(cache.head.load(Ordering::SeqCst) == 0);
    }
}