use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::thread::JoinHandle;
use std::sync::Arc;
use std::ops::Deref;
use crate::traits::*;

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
		alloc!(SPSCNode(AtomicPtr::new(ptr::null_mut()), AtomicPtr::new(next)))
	}

	fn make_ring(size:usize) -> *mut SPSCNode<T> {
		assert!(size >= 2);
		let tail = SPSCNode::<T>::new_ptr(ptr::null_mut());
		let mut head = SPSCNode::<T>::new_ptr(tail);
		for _ in 0..(size-2) {
			head = SPSCNode::<T>::new_ptr(head);
		}
		unsafe {
			tail.as_ref().unwrap().1.store(head, Ordering::SeqCst);
			return head;
		}
	}
}

// non-growable spsc queue 
#[derive(Debug)]
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
						self.tail.store(tail_ref.1.load(Ordering::SeqCst), Ordering::SeqCst);
						return true;
					},
					Err(_) => return false
				}
			}
		} else {
			unsafe {
				let tail_ref = tail.as_ref().unwrap();
				tail_ref.0.store(ptr, Ordering::SeqCst);
				self.tail.store(tail_ref.1.load(Ordering::SeqCst), Ordering::SeqCst);
				return true;				
			}

		}
	}

	pub fn pop(&self) -> Option<*mut T> {
		let head = self.head.load(Ordering::SeqCst);
		unsafe {
			let head_ref = head.as_ref().unwrap();
			let read_ptr = head_ref.0.swap(ptr::null_mut(), Ordering::SeqCst);
			if read_ptr == ptr::null_mut() {
				return None;
			} else {
				// advance only if pop worked
				self.head.store(head_ref.1.load(Ordering::SeqCst), Ordering::SeqCst);
				return Some(read_ptr);
			}
		}
	}
}

// Used as a switch to communicate when a thread should shut down
#[derive(Clone, Debug)]
pub struct Switch(Arc<AtomicBool>);

impl NewType for Switch {
    fn new() -> Self {
        Switch(Arc::new(AtomicBool::new(false)))
    }
}

impl Switch {   
    pub fn set(&self, state:bool) {
        self.0.store(state, Ordering::SeqCst);
    }
    
    pub fn get(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

// Generic wrapper for shared reference between threads
#[derive(Debug)]
pub struct TVal<T>(Arc<T>);

impl<T> TVal<T> {
    pub fn new(val: T) -> Self {
        TVal(Arc::new(val))
    }
}

impl<T> Clone for TVal<T> {
    fn clone(&self) -> Self {
        TVal(self.0.clone())
    }
}

impl<T> Deref for TVal<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ExecUnit<T> {
	handle:AtomicPtr<JoinHandle<()>>,
	switch:Switch,
	queue:TVal<SpSc<T>>
}

impl<T: 'static> ExecUnit<T> {
    pub fn new(qsize:usize) -> ExecUnit<T> {
        ExecUnit{handle:AtomicPtr::new(ptr::null_mut()),
                 switch:Switch::new(),
                 queue:TVal::new(SpSc::new(qsize))}
    }

    pub fn start(&self, func:fn(*mut T)) {
    	self.switch.set(true);
    	let tswitch = self.switch.clone();
    	let tqueue = self.queue.clone();
    	self.handle.store(alloc!(thread::spawn({move ||
    		loop {
    			if !tswitch.get() {
    				break;
    			}
    			match tqueue.pop() {
    				Some(ptr) => func(ptr),
    				None => ()
    			}
    			// todo parking
    		}
    	})), Ordering::SeqCst);
    }

    pub fn stop(&self) {
    	
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn make_ring_works() {
    	let ring_size = 5;
        let ring = SPSCNode::<TestType>::make_ring(ring_size);
        let mut ring_ptr = ring;
        for _ in 0..ring_size {
        	ring_ptr = unsafe { ring_ptr.as_ref().unwrap().1.load(Ordering::SeqCst) };
        }
        assert_eq!(ring, ring_ptr);
    }

    #[test]
    fn spsc_push_works() {
    	let qsize = 3;
    	let queue = SpSc::<TestType>::new(qsize);
    	let items = [alloc!(TestType(4)), alloc!(TestType(4)), alloc!(TestType(4)), alloc!(TestType(4))];
    	for i in 0..qsize {
    		assert!(queue.push(items[i]));
    	}
    	// this should fail
    	assert!(!queue.push(items[3]));
    	for j in 0..qsize {
    		free!(items[j]);
    	}
    }

    #[test]
    fn spsc_is_full_works() {
    	let qsize = 3;
    	let queue = SpSc::<TestType>::new(qsize);
    	let items = [alloc!(TestType(4)), alloc!(TestType(4)), alloc!(TestType(4))];
    	for i in 0..qsize {
    		assert!(queue.push(items[i]));
    	}

    	assert!(queue.is_full());
    	for j in 0..qsize {
    		free!(items[j]);
    	}
    }

    #[test]
    fn spsc_pop_works() {
    	let qsize = 3;
    	let queue = SpSc::<TestType>::new(qsize);
    	let items = [alloc!(TestType(4)), alloc!(TestType(4)), alloc!(TestType(4)), alloc!(TestType(4))];
    	for i in 0..qsize {
    		assert!(queue.push(items[i]));
    	}
    	// this should fail
    	assert!(!queue.push(items[3]));
    	for _ in 0..qsize {
    		match queue.pop() {
    			Some(ptr) => free!(ptr),
    			None => panic!("Pop with non empty queue failed")
    		}
    	}
    	match queue.pop() {
    		Some(ptr) => panic!("Expected empty queue but got {:?}", ptr),
    		_ => ()
    	}
    }

    #[test]
    fn switch_works() {
	    let a = Switch::new();
	    let b = a.clone();
	    let handler = thread::spawn({move || 
	        loop {
	            if b.get() {
	                b.set(false);
	                break;
	            }
	        }
	    });
	    a.set(true);
	    handler.join().unwrap();
	    assert!(!a.get());
    }
}
