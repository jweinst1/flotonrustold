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
	pub fn create_seq(&self, seq:usize) {
		match seq {
			0 => self.create_0(),
			1 => self.create_1(),
			2 => self.create_0().create_1(),
			3 => self.create_1().create_1(),
			4 => self.create_0().create_0().create_1(),
			_ => {
				// todo
				let count = seq.count_ones();
				let mut place = 0;
				
			}
		}
	}
}

// A trie that uses integers as keys
#[derive(Debug)]
pub struct IntTrie<T>{
	nodes:IntNode<T>
}

/*
impl<T> IntTrie<T> {
	pub fn create(size:usize) -> IntTrie<T> {
		let base = IntNode::new();
		match size {
			0 => break,
			1 => base.create_0(),
			2 => {
				base.create_0();
				base.create_1();
			}
		}
		IntTrie{nodes:base}
	}
}*/