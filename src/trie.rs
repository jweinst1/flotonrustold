use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::ptr;
use crate::traits::*;

#[derive(Debug)]
struct IntNode<T>(AtomicPtr<T>, [AtomicPtr<IntNode<T>>;2]);

impl<T> NewType for IntNode<T> {
	fn new() -> Self {
		IntNode(newptr!(), [newptr!(), newptr!()])
	}
}

impl<T> IntNode<T> {
	// little endian style
	#[inline]
	pub fn get_0(&self) -> &IntNode<T> {
		let got_ptr = self.1[0].load(Ordering::SeqCst);
		if isnull!(got_ptr) {
			let new_node = alloc!(IntNode::new());
			match self.1[0].compare_exchange(ptr::null_mut(), new_node, Ordering::SeqCst, Ordering::SeqCst) {
				Ok(_) => return ptref!(new_node),
				Err(fptr) => {
					free!(new_node);
					return ptref!(fptr);
				}
			}
		} else {
			return ptref!(got_ptr);
		}
	}

	#[inline]
	pub fn get_1(&self) -> &IntNode<T> {
		let got_ptr = self.1[1].load(Ordering::SeqCst);
		if isnull!(got_ptr) {
			let new_node = alloc!(IntNode::new());
			match self.1[1].compare_exchange(ptr::null_mut(), new_node, Ordering::SeqCst, Ordering::SeqCst) {
				Ok(_) => return ptref!(new_node),
				Err(fptr) => {
					free!(new_node);
					return ptref!(fptr);
				}
			}
		} else {
			return ptref!(got_ptr);
		}
	}

	pub fn get_seq(&self, seq:usize) -> &IntNode<T> {
		match seq {
			0 => self.get_0(),
			1 => self.get_1(),
			2 => self.get_0().get_1(),
			3 => self.get_1().get_1(),
			4 => self.get_0().get_0().get_1(),
			5 => self.get_1().get_0().get_1(),
			6 => self.get_0().get_1().get_1(),
			7 => self.get_1().get_1().get_1(),
			8 => self.get_0().get_0().get_0().get_1(),
			9 => self.get_1().get_0().get_0().get_1(),
			10 => self.get_0().get_1().get_0().get_1(),
			11 => self.get_1().get_1().get_0().get_1(),
			12 => self.get_0().get_0().get_1().get_1(),
			_ => {
				let limit = usize::BITS - seq.leading_zeros();
				let mut refs = self;
				for i in 0..limit {
					match (seq >> i) & 1 {
						0 => refs = refs.get_0(),
						1 => refs = refs.get_1(),
						_ => unreachable!()
					}	
				}
				refs
			}
		}
	}
	// optimized function that assumes child
	// not yet created
	#[inline]
	pub fn create_0(&self) -> &IntNode<T> {
		let new_node = alloc!(IntNode::new());
		self.1[0].store(new_node, Ordering::SeqCst);
		ptref!(new_node)
	}

	#[inline]
	pub fn create_1(&self) -> &IntNode<T> {
		let new_node = alloc!(IntNode::new());
		self.1[1].store(new_node, Ordering::SeqCst);
		ptref!(new_node)
	}

	// Creates a sequence of nodes in little endian order
	pub fn create_seq(&self, seq:usize) -> &IntNode<T> {
		match seq {
			0 => self.create_0(),
			1 => self.create_1(),
			2 => self.create_0().create_1(),
			3 => self.create_1().create_1(),
			4 => self.create_0().create_0().create_1(),
			5 => self.create_1().create_0().create_1(),
		    6 => self.create_0().create_1().create_1(),
		    7 => self.create_1().create_1().create_1(),
		    8 => self.create_0().create_0().create_0().create_1(),
		    9 => self.create_1().create_0().create_0().create_1(),
		    10 => self.create_0().create_1().create_0().create_1(),
			_ => {
				let limit = usize::BITS - seq.leading_zeros();
				let mut refs = self;
				for i in 0..limit {
					match (seq >> i) & 1 {
						0 => refs = refs.create_0(),
						1 => refs = refs.create_1(),
						_ => unreachable!()
					}
				}
				self
			}
		}
	}
}

// A trie that uses integers as keys
#[derive(Debug)]
pub struct IntTrie<T>{
	nodes:IntNode<T>
}


impl<T> IntTrie<T> {
	// Pre-creates up to size, the amount of slots
	// size of 1 means only the first, '0' slot is pre-
	// created.
	pub fn new(size:usize) -> IntTrie<T> {
		let base = IntNode::new();
		match size {
			0 => (),
			1 => { base.create_0(); },
			2 => {
				base.create_seq(0);
				base.create_seq(1);
			},
			3 => {
				base.create_seq(0);
				base.create_seq(1);
				base.create_seq(2);				
			},
			4 => {
				base.create_seq(0);
				base.create_seq(1);
				base.create_seq(2);
				base.create_seq(3);			
			},
			5 => {
				base.create_seq(0);
				base.create_seq(1);
				base.create_seq(2);
				base.create_seq(3);
				base.create_seq(4);
			},
			6 => {
				base.create_seq(0);
				base.create_seq(1);
				base.create_seq(2);
				base.create_seq(3);
				base.create_seq(4);
				base.create_seq(5);			
			}
			_ => {
				for i in 0..size {
					base.create_seq(i);
				}
			}
		}
		IntTrie{nodes:base}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inode_create01_works() {
    	let a = IntNode::<u32>::new();
    	let created = a.create_0();
    	let created1 = a.create_1();
    	assert!(nonull!(a.1[0].load(Ordering::SeqCst)));
    	assert!(nonull!(a.1[1].load(Ordering::SeqCst)));
    	assert!(isnull!(created.1[0].load(Ordering::SeqCst)));
    	assert!(isnull!(created.1[1].load(Ordering::SeqCst)));
    	assert!(isnull!(created1.1[0].load(Ordering::SeqCst)));
    	assert!(isnull!(created1.1[1].load(Ordering::SeqCst)));
    	free!(a.1[0].load(Ordering::SeqCst));
    	free!(a.1[1].load(Ordering::SeqCst));
    }

    #[test]
    fn inode_get01_works() {
    	let a = IntNode::<u32>::new();
    	let got0 = a.get_0();
    	let got1 = a.get_1();
    	assert!(nonull!(a.1[0].load(Ordering::SeqCst)));
    	assert!(nonull!(a.1[1].load(Ordering::SeqCst)));
    	assert!(isnull!(got0.1[0].load(Ordering::SeqCst)));
    	assert!(isnull!(got0.1[1].load(Ordering::SeqCst)));
    	assert!(isnull!(got1.1[0].load(Ordering::SeqCst)));
    	assert!(isnull!(got1.1[1].load(Ordering::SeqCst)));
    	free!(a.1[0].load(Ordering::SeqCst));
    	free!(a.1[1].load(Ordering::SeqCst));
    }

    #[test]
    fn inode_create_seq_works() {
    	let b = IntNode::<u32>::new();
    	b.create_seq(2);
    	unsafe {
    		let node2 = b.1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			.1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	b.create_seq(3);
    	unsafe {
    		let node2 = b.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			.1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	let c = IntNode::<u32>::new();
    	b.create_seq(4);
    	unsafe {
    		let node2 = b.1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	b.create_seq(7);
    	unsafe {
    		let node2 = b.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    }
}

