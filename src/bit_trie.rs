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

pub struct BitTrie<T> {
	base:AtomicPtr<BitNode<T>>
}

// todo Drop

impl<T> BitTrie<T> {
	pub fn new() -> BitTrie<T> {
		BitTrie{base:AtomicPtr::new(Box::into_raw(Box::new(BitNode::new())))}
	}

	pub fn find_u32(&self, key:u32) -> Option<*const T> {
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
                    	return Some(got_ptr);
                    } else {
                    	return None;
                    }
				},
				None => { return None; }
			}
	    }		
	}

	pub fn insert_32(&self, key:u32, val:T) -> bool {
		let mut cur_ptr = self.base.load(Ordering::SeqCst);
		for i in 0..32 {
			unsafe {
				match cur_ptr.as_ref() {
					Some(r) => {
						cur_ptr = r.create_child(((key >> i) & 1) as usize);
					},
					None => { panic!("Expected child bit trie node, got nullptr!"); }
				}
			}
		}
		unsafe {
			match cur_ptr.as_ref() {
				Some(r) => {
					let incoming_ptr = Box::into_raw(Box::new(val));
					match r.value.compare_exchange(ptr::null_mut(), incoming_ptr, Ordering::SeqCst, Ordering::SeqCst) {
						Ok(_) => { return true; },
						Err(_) => {
							unsafe { drop(Box::from_raw(incoming_ptr)); }
							return false;
						}
					}
				},
				None => { panic!("Expected pointer at end of 32bit trie isnert!"); }
			}
	    }
	}
}



#[cfg(test)]
mod tests {
    use super::*;

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
    fn find_32_works() {
    	let t1 = BitTrie::<i32>::new();
    	assert!(t1.insert_32(5381, 2000));
    	match t1.find_u32(5381) {
    		Some(p) => {
    			assert!(p != ptr::null_mut());
    			unsafe {
    				assert!(*p == 2000);
    			}
    		},
    		None => {
    			panic!("Expected pointer to be found in bit trie!");
    		}
    	}
    }

    #[test]
    fn double_insert_fails() {
    	let t1 = BitTrie::<i32>::new();
    	assert!(t1.insert_32(5381, 2000));
    	assert!(!t1.insert_32(5381, 3000));
    }
}

