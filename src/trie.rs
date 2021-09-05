use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::ptr;
use crate::traits::*;
use crate::tlocal;

#[derive(Debug)]
pub struct IntNode<T>(AtomicPtr<T>, [AtomicPtr<IntNode<T>>;2]);

impl<T> NewType for IntNode<T> {
	fn new() -> Self {
		IntNode(newptr!(), [newptr!(), newptr!()])
	}
}

impl<T: NewType> IntNode<T> {
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

	pub fn check_if_one<P>(&self, func:fn(&T, &P) -> bool, arg:&P) -> bool {
		unsafe {
			match self.0.load(Ordering::SeqCst).as_ref() {
				Some(r) => {
					if func(r, arg) {
						return true;
					}
				},
				None => ()
			}
			match self.1[0].load(Ordering::SeqCst).as_ref() {
				Some(r) => {
					if r.check_if_one(func, arg) {
						return true;
					}
				},
				_ => ()
			}
			match self.1[1].load(Ordering::SeqCst).as_ref() {
				Some(r) => {
					if r.check_if_one(func, arg) {
						return true;
					}
				},
				_ => ()
			}
		}
		return false;
	}
}

// A trie that uses integers as keys
#[derive(Debug)]
pub struct IntTrie<T>{
	nodes:IntNode<T>
}


impl<T: NewType> IntTrie<T> {
	// Pre-creates up to size, the amount of slots
	// size of 1 means only the first, '0' slot is pre-
	// created.
	// todo optimize get based on pre-created values
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
			},
			7 => {
				base.create_seq(0);
				base.create_seq(1);
				base.create_seq(2);
				base.create_seq(3);
				base.create_seq(4);
				base.create_seq(5);
				base.create_seq(6);	
			}
			_ => {
				for i in 0..size {
					base.create_seq(i);
				}
			}
		}
		IntTrie{nodes:base}
	}

	pub fn get_node(&self, key:usize) -> &IntNode<T> {
		self.nodes.get_seq(key)
	}

	// Safety guarantee that is only accessed by owning thread
	#[inline]
	pub fn get_by_tid(&self) -> &T {
		let node = self.nodes.get_seq(tlocal::tid());
		let loaded = node.0.load(Ordering::SeqCst);
		if isnull!(loaded) {
			let init = alloc!(T::new());
			node.0.store(init, Ordering::SeqCst);
			ptref!(init)
		}
		else {
			ptref!(loaded)
		}
	} 

	#[inline]
	pub fn check_if_one<P>(&self, func:fn(&T, &P) -> bool, arg:&P) -> bool {
		self.nodes.check_if_one(func, arg)
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    impl NewType for u32 {
    	fn new() -> Self { 0 }
    }

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
    	c.create_seq(4);
    	unsafe {
    		let node2 = c.1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	c.create_seq(7);
    	unsafe {
    		let node2 = c.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	let d = IntNode::<u32>::new();
    	d.create_seq(10);
    	unsafe {
     		let node2 = d.1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	d.create_seq(17);
    	unsafe {
     		let node2 = d.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    }

    #[test]
    fn inode_get_seq_works() {
    	let b = IntNode::<u32>::new();
    	b.get_seq(3);
    	unsafe {
    		let node2 = b.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			.1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	b.get_seq(7);
    	unsafe {
    		let node2 = b.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	b.get_seq(6);
    	unsafe {
    		let node2 = b.1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    	b.get_seq(21);
    	unsafe {
     		let node2 = b.1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[0].load(Ordering::SeqCst).as_ref().unwrap()
    			         .1[1].load(Ordering::SeqCst).as_ref().unwrap();
    		assert!(isnull!(node2.1[0].load(Ordering::SeqCst)));
    		assert!(isnull!(node2.1[1].load(Ordering::SeqCst)));
    	}
    }

    #[derive(Debug)]
    struct TimePoint(AtomicUsize);

    impl NewType for TimePoint {
    	fn new() -> Self { TimePoint(AtomicUsize::new(0)) }
    }

    #[derive(Debug)]
    struct TimeRange(usize, usize);

    impl TimeRange {
    	fn contains(&self, arg:usize) -> bool {
    		self.0 <= arg && self.1 >= arg
    	}
    }

    impl TimePoint {
    	fn time(&self) -> usize {
    		self.0.load(Ordering::SeqCst)
    	}
    }

    fn is_over_time(obj:&TimePoint, op:&usize) -> bool {
    	obj.time() > *op
    }

    fn is_between_time(obj:&TimePoint, op:&TimeRange) -> bool {
    	op.contains(obj.time())
    }

    #[test]
    fn check_if_one_works() {
    	// integer cmp test
    	let b = IntTrie::<TimePoint>::new(4);
    	let got = b.get_node(3);
    	got.0.store(alloc!(TimePoint(AtomicUsize::new(6))), Ordering::SeqCst);
    	let got2 = b.get_node(1);
    	got2.0.store(alloc!(TimePoint(AtomicUsize::new(2))), Ordering::SeqCst);
    	assert!(b.check_if_one(is_over_time, &4));
    	// range test
    	let rng = TimeRange(0, 2);
    	assert!(b.check_if_one(is_between_time, &rng));
    }

    #[test]
    fn check_if_get_by_tid_works() {
    	let b = IntTrie::<TimePoint>::new(4);
    	let current_tid = tlocal::tid();
    	let _val = b.get_by_tid();
    	let regular_node = b.get_node(current_tid);
    	assert!(nonull!(regular_node.0.load(Ordering::SeqCst)));
    }
}
