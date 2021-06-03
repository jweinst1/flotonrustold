use std::sync::atomic::{AtomicI64, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;
use crate::bit_trie::BitTrie;


struct List<T> {
	key_size:usize,
	items:BitTrie<SharedPtr<T>>,
	len:AtomicI64
}

impl<T> List<T> {
	pub fn new(key_size:usize) -> List<T> {
		List{key_size:key_size, items:BitTrie::new(), len:AtomicI64::new(0)}
	}

	pub fn size(&self) -> i64 {
		self.len.load(Ordering::SeqCst)
	}
	// append item to end of list
	pub fn push(&self, value:T, attempts:usize) -> bool {
		for i in 0..attempts {
			let cur_len = self.len.fetch_add(1, Ordering::SeqCst);
			if cur_len < 0 {
				continue;
			}
			let slot = self.items.insert(cur_len as u64, self.key_size, SharedPtr::new(None));
			if slot.reset(Some(value), attempts - i) {
				return true;
			} else {
				// if we fail to reset, lets pull back the length.
				self.len.fetch_sub(1, Ordering::SeqCst);
				break;
			}
		}
		return false;
	}
	// remove last item in list
	pub fn pop(&self, attempts:usize) -> bool {
		let observed = self.len.fetch_sub(1, Ordering::SeqCst);
		if (observed - 1) < 0	{
			self.len.fetch_add(1, Ordering::SeqCst);
			// Popping already took place
			return false;
		}
		match self.items.find((observed - 1) as u64, self.key_size) {
			Some(p) => { return p.reset(None, attempts); },
		    None => { return false; /* maybe panic ?*/ }
		}
	}

	pub fn get(&self, index:u64, attempts:usize) -> Option<SharedPtr<T>> {
		for _ in 0..attempts {
			if (index as i64) < self.len.load(Ordering::SeqCst) {
				match self.items.find(index, self.key_size) {
					Some(r) => { 
						let local = r.clone();
						if r.valid() {
							return Some(local);
						}
					},
					None => { return None; }
				}
			} else {
				return None; // todo differentiate length too small
			}
		}
		return None;
	}

	pub fn set(&self, index:u64, value:T, attempts:usize) -> bool {
		for _ in 0..attempts {
			if (index as i64) < self.len.load(Ordering::SeqCst) {
				match self.items.find(index, self.key_size) {
					Some(r) => { 
						return r.reset(Some(value), attempts);
					},
					None => { continue; }
				}
			} else {
				return false;
			}
		}
		return false;
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_get_fails() {
    	let lst = List::<i32>::new(16);
    	match lst.get(0, 1) {
    		Some(unexpected) => { panic!("Expected None return from empty get, but got {:?}", unexpected); },
    		None => ()
    	}
    }

    #[test]
    fn empty_set_fails() {
    	let lst = List::<i32>::new(16);
    	assert!(!lst.set(0, 10, 1));
    }

    #[test]
    fn empty_pop_fails() {
    	let lst = List::<i32>::new(16);
    	assert!(!lst.pop(1));
    	assert!(lst.size() == 0);
    }

    #[test]
    fn push_works() {
    	let lst = List::<i32>::new(16);
    	assert!(lst.push(5, 1));
    	match lst.get(0, 1) {
    		Some(g) => {
    			let local = g.clone();
    			assert!(local.valid());
    			unsafe { assert!(*(local.get().as_ref().unwrap()) == 5); }
    		},
    		None => { panic!("Expected 5, did not find pointer in bit trie inside list"); }
    	}
    }

    #[test]
    fn set_works() {
    	let lst = List::<i32>::new(16);
    	assert!(lst.push(5, 1));
    	assert!(lst.set(0, 333, 1));
    	match lst.get(0, 1) {
    		Some(g) => {
    			assert!(g.valid());
    			assert!(g.count().unwrap() == 2); // get returns a clone
    			unsafe { assert!(*(g.get().as_ref().unwrap()) == 333); }
    		},
    		None => { panic!("Expected 333, did not find pointer in bit trie inside list"); }
    	}
    }

    #[test]
    fn set_destroys_old_object() {
    	let lst = List::<i32>::new(16);
    	assert!(lst.push(5, 1));
    	match lst.get(0, 1) {
    		Some(g) => {
    			assert!(lst.set(0, 10, 1));
    			assert!(g.count().unwrap() == 1);
    		},
    		None => { panic!("Expected 5, did not find pointer in bit trie inside list"); }
    	}
    }
}
