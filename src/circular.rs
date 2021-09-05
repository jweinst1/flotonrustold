use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;

#[derive(Debug)]
pub struct CircleNode<T>(T, AtomicPtr<CircleNode<T>>);

impl<T: Clone> CircleNode<T> {
	#[inline]
	pub fn new_ptr(val:&T) -> *mut CircleNode<T> {
		alloc!(CircleNode(val.clone(), newptr!()))
	}

	pub fn make_ring(val:&T, count:usize) -> *mut CircleNode<T> {
		assert!(count >= 2);
		let base = CircleNode::new_ptr(val);
		let mut cur = base;
		for _ in 0..(count-1) {
			let next_ptr = CircleNode::new_ptr(val);
			ptref!(cur).1.store(next_ptr, Ordering::SeqCst);
			cur = next_ptr;
		}
		ptref!(cur).1.store(base, Ordering::SeqCst);
		base
	}

	pub fn get_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

#[derive(Debug)]
pub struct CircleList<T>(AtomicPtr<CircleNode<T>>, AtomicUsize);

impl<T: Clone> CircleList<T> {
	pub fn new(val:&T, size:usize) -> CircleList<T> {
		CircleList(AtomicPtr::new(CircleNode::make_ring(val, size)), AtomicUsize::new(size))
	}

	pub fn next(&self) -> &T {
		let cur = self.0.load(Ordering::SeqCst);
		unsafe {
			let rcur = cur.as_ref().unwrap();
			self.0.store(rcur.1.load(Ordering::SeqCst), Ordering::SeqCst);
			return &rcur.0;
		}
	}

	pub fn next_ptr(&self) -> *mut CircleNode<T> {
		let cur = self.0.load(Ordering::SeqCst);
		unsafe {
			let rcur = cur.as_ref().unwrap();
			self.0.store(rcur.1.load(Ordering::SeqCst), Ordering::SeqCst);
			return cur;
		}
	}

	pub fn len(&self) -> usize {
		self.1.load(Ordering::SeqCst)
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestType(u32);

    #[test]
    fn make_ring_works() {
    	let template = TestType(30);
    	let ring = CircleNode::make_ring(&template, 10);
    	let mut cur_ptr = ring;
    	// do one lap, check it's 10 in length
    	for _ in 0..10 {
    		cur_ptr = ptref!(cur_ptr).1.load(Ordering::SeqCst);
    	}
    	assert_eq!(cur_ptr, ring);
    }

    #[test]
    fn circ_list_next_works() {
    	let base = TestType(7);
    	let mut list = CircleList::new(&base, 10);
    	unsafe {
    		(*list.0.load(Ordering::SeqCst)).0 = TestType(1);
    	}
    	for _ in 0..10 {
    		list.next();
    	}
    	assert_eq!(list.next().0, 1);
    }
}