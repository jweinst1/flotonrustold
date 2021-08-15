use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};

const THREAD_COUNT_DEFAULT:usize = 8;
// Considered the maximum amount of threads
static THREAD_COUNT:AtomicUsize = AtomicUsize::new(THREAD_COUNT_DEFAULT);
// Used to gradually pass thread id to new threads
static LAST_THREAD_ID:AtomicUsize = AtomicUsize::new(0);

pub fn make_thread_id() -> usize {
	if THREAD_COUNT.load(Ordering::SeqCst) == LAST_THREAD_ID.load(Ordering::SeqCst) {
		panic!("Attempted to increase thread id past maximum {:?}", THREAD_COUNT.load(Ordering::SeqCst));
	} else {
		LAST_THREAD_ID.fetch_add(1, Ordering::SeqCst)
	}
}

pub fn get_thread_count() -> usize {
    THREAD_COUNT.load(Ordering::SeqCst)
}

pub fn set_thread_count(count:usize) {
    THREAD_COUNT.store(count, Ordering::SeqCst);
}

#[derive(Debug)]
struct SPSCNode<T>(AtomicPtr<T>, AtomicPtr<SPSCNode<T>>);

impl<T> SPSCNode<T> {
	fn new_ptr(next:*mut SPSCNode<T>) -> *mut SPSCNode<T> {
		Box::into_raw(Box::new(SPSCNode(AtomicPtr::new(ptr::null_mut()), AtomicPtr::new(next))))
	}

	fn make_ring(size:usize) -> *mut SPSCNode<T> {
		let tail = SPSCNode::<T>::new_ptr(ptr::null_mut());
		let mut head = SPSCNode::<T>::new_ptr(tail);
		for _ in 0..size {
			head = SPSCNode::<T>::new_ptr(head);
		}
		unsafe {
			tail.as_ref().unwrap().1.store(head, Ordering::SeqCst);
			return head;
		}
	}
}

// non-growable spsc queue 
pub struct SpSc<T> {
	head:AtomicPtr<SPSCNode<T>>,
	tail:AtomicPtr<SPSCNode<T>>
}

impl<T> SpSc<T> {
	pub fn new(size:usize) -> SpSc<T> {
		let ring = SPSCNode::<T>::make_ring(size);
		SpSc{head:AtomicPtr::new(ring), tail:AtomicPtr::new(ring)}
	}

	pub fn is_full(&self) -> bool {
		let head = self.head.load(Ordering::SeqCst);
		let tail = self.tail.load(Ordering::SeqCst);
		unsafe {
			head == tail && tail.as_ref().unwrap().0.load(Ordering::SeqCst) != ptr::null_mut()
		}
	}

	pub fn push(&self, ptr:*mut T) -> bool {
		let head = self.head.load(Ordering::SeqCst);
		let tail = self.tail.load(Ordering::SeqCst);
		if head == tail {
			unsafe {
				let tail_ref = tail.as_ref().unwrap();
				match tail_ref.0.compare_exchange(ptr::null_mut(), ptr, 
					                                            Ordering::SeqCst, Ordering::SeqCst) {
					Ok(_) => {
						self.tail.store(tail_ref.1.load(Ordering::SeqCst);
						return true;
					}
					Err(_) => return false
				}
			}
		} else {
			let tail_ref = tail.as_ref().unwrap();
			tail_ref.0.store(ptr, Ordering::SeqCst);
			self.tail.store(tail_ref.1.load(Ordering::SeqCst));
			return true;
		}
	}

	pub fn pop(&self) -> Option<*mut T> {
		let head = self.head.load(Ordering::SeqCst);
		let tail = self.tail.load(Ordering::SeqCst);
		if head == tail {

		} else {
			
		}	
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn freenode_works() {
        unimplemented!();
    }
}
