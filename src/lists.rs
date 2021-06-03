use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;
use crate::bit_trie::BitTrie;


struct List<T> {
	key_size:usize,
	items:BitTrie<SharedPtr<T>>,
	len:AtomicU64
}

impl<T> List<T> {
	pub fn new(key_size:usize) -> List<T> {
		List{key_size:key_size, items:BitTrie::new(), len:AtomicU64::new(0)}
	}
	// append item to end of list
	pub fn append(&self, value:T, attempts:usize) -> bool {
		let slot = self.items.insert(self.len.fetch_add(1, Ordering::SeqCst), self.key_size, SharedPtr::new(None));
		return slot.reset(Some(value), attempts);
	}
	// remove last item in list
	pub fn pop(&self, attempts:usize) -> bool {
		for _ in 0..attempts {
			let cur_len = self.len.load(Ordering::SeqCst);
			if cur_len > 0 {
				let desired = cur_len - 1;
				match self.len.compare_exchange(cur_len, desired, Ordering::SeqCst, Ordering::SeqCst) {
					Ok(_) => { return true; },
					Err(_) => { return false; }
				}
			} else {
				return false;
			}
		}
		return false;
	}

	pub fn get(&self, index:u64, attempts:usize) -> Option<SharedPtr<T>> {
		for _ in 0..attempts {
			if index < self.len.load(Ordering::SeqCst) {
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
			if index < self.len.load(Ordering::SeqCst) {
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
}
