use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;

struct BitNode<T> {
	value:AtomicPtr<T>,
	children:[AtomicPtr<BitNode<T>>;2]
}

impl<T> BitNode<T> {
	fn new() -> BitNode<T> {
		BitNode{value:AtomicPtr::new(ptr::null_mut()), 
			    children:[AtomicPtr::new(ptr::null_mut()),
			              AtomicPtr::new(ptr::null_mut())]}
	}

	fn get(&self) -> *mut T {
		self.value.load(Ordering::SeqCst)
	}

	fn get_child(&self, idx:usize) -> *mut BitNode<T> {
		self.children[idx].load(Ordering::SeqCst)
	}

	fn create_child(&self, idx:usize) -> *mut BitNode<T> {
		let input_child = Box::into_raw(Box::new(BitNode::new()));
		match self.children[idx].compare_exchange(ptr::null_mut(), input_child, Ordering::SeqCst, Ordering::SeqCst) {
			Ok(_) => self.children[idx].load(Ordering::SeqCst),
			Err(_) => {
				unsafe { drop(Box::from_raw(input_child)); }
				self.children[idx].load(Ordering::SeqCst)
			}
		}
	}
}

impl<T> Drop for BitNode<T> {
	// not thread safe, this should not be dropped in a multi-threaded context
    fn drop(&mut self) {
    	let ptr_val = self.get();
    	let ptr_0 = self.get_child(0);
    	let ptr_1 = self.get_child(1);
    	if ptr_val != ptr::null_mut() {
    		unsafe { drop(Box::from_raw(ptr_val)); }
    	}

    	if ptr_0 != ptr::null_mut() {
    		unsafe { drop(Box::from_raw(ptr_0)); }
    	}

    	if ptr_1 != ptr::null_mut() {
    		unsafe { drop(Box::from_raw(ptr_1)); }
    	}
    }
}

pub struct BitTrie<T> {
	base:AtomicPtr<BitNode<T>>
}

impl<T> Drop for BitTrie<T> {
	// not thread safe, this should not be dropped in a multi-threaded context
    fn drop(&mut self) {
    	let base_ptr = self.base.load(Ordering::SeqCst);
    	if base_ptr != ptr::null_mut() {
    		unsafe { drop(Box::from_raw(base_ptr)); }
    	}
    }
}

impl<T> BitTrie<T> {
	pub fn new() -> BitTrie<T> {
		BitTrie{base:AtomicPtr::new(Box::into_raw(Box::new(BitNode::new())))}
	}

	pub fn find_u32(&self, key:u32) -> Option<& T> {
		let mut cur_ptr = self.base.load(Ordering::SeqCst);
		for i in 0..32 {
			unsafe {
				match cur_ptr.as_ref() {
					Some(r) => {
						cur_ptr = r.get_child(((key >> i) & 1) as usize);
					},
					None => { 
						// not found
						return None;
					}
				}
			}
		}
		unsafe {
			match cur_ptr.as_ref() {
				Some(r) => {
					let got_ptr = r.value.load(Ordering::SeqCst);
                    if got_ptr != ptr::null_mut() {
                    	return Some(got_ptr.as_ref().unwrap());
                    } else {
                    	return None;
                    }
				},
				None => { return None; }
			}
	    }		
	}

	pub fn insert_32(&self, key:u32, val:T) -> &T {
		let mut cur_ptr = self.base.load(Ordering::SeqCst);
		for i in 0..32 {
			unsafe {
				match cur_ptr.as_ref() {
					Some(r) => {
						cur_ptr = r.create_child(((key >> i) & 1) as usize);
					},
					// create child always returns a valid pointer, if not, something is VERY wrong
					None => { panic!("Expected child bit trie node, got nullptr!"); }
				}
			}
		}
		unsafe {
			match cur_ptr.as_ref() {
				Some(r) => {
					let incoming_ptr = Box::into_raw(Box::new(val));
					match r.value.compare_exchange(ptr::null_mut(), incoming_ptr, Ordering::SeqCst, Ordering::SeqCst) {
						Ok(_) => { return incoming_ptr.as_ref().unwrap(); },
						Err(_) => {
							drop(Box::from_raw(incoming_ptr));
							return r.get().as_ref().unwrap();
						}
					}
				},
				None => { panic!("Expected pointer at end of 32bit trie insert!"); }
			}
	    }
	}
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn create_get_child_works() {
    	let n1 = BitNode::<i32>::new();
    	assert!(n1.create_child(0) != ptr::null_mut());
    	unsafe {
    		let child = n1.get_child(0).as_ref().unwrap();
    		assert!(child.get_child(0) == ptr::null_mut());
    		assert!(child.get_child(1) == ptr::null_mut());
    	}
    }

    #[test]
    fn insert_32_works() {
    	let t1 = BitTrie::<i32>::new();
    	let got = t1.insert_32(5381, 2000);
    	assert!(*got == 2000);
    }

    #[test]
    fn find_32_works() {
    	let t1 = BitTrie::<i32>::new();
    	let got = t1.insert_32(5381, 2000);
    	match t1.find_u32(5381) {
    		Some(p) => {
    			assert!(*p == 2000);
    		},
    		None => {
    			panic!("Expected pointer to be found in bit trie!");
    		}
    	}
    }

    #[test]
    fn double_insert_fails() {
    	let t1 = BitTrie::<i32>::new();
    	let got1 = t1.insert_32(5381, 2000);
    	let got2 = t1.insert_32(5381, 3000);
    	// no over writes allowed
    	assert!(*got2 == 2000);
    }

    #[test]
    fn bit_node_drop_works() {
    	let a1 = Arc::new(30);
    	{
    		let a2 = a1.clone();
    		let n1 = BitNode::<Arc<i32>>::new();
    		assert!(Arc::strong_count(&a2) == 2);
    		n1.value.store(Box::into_raw(Box::new(a2)), Ordering::SeqCst);
    	}
    	assert!(Arc::strong_count(&a1) == 1);
    }

    #[test]
    fn bit_trie_drop_works() {
    	let a1 = Arc::new(30);
    	{
    		let a2 = a1.clone();
    		let a3 = a1.clone();
    		let t1 = BitTrie::<Arc<i32>>::new();
    		assert!(Arc::strong_count(&a2) == 3);
    		t1.insert_32(8899, a2);
    		t1.insert_32(8894, a3);
    	}
    	assert!(Arc::strong_count(&a1) == 1);
    }
}

