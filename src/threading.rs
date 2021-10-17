use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::time::Duration;
use std::thread::JoinHandle;
use std::sync::Arc;
use std::ops::Deref;
use crate::circular::CircleList;
use crate::traits::*;

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
	tail:AtomicPtr<SPSCNode<T>>,
    pub size:usize
}

impl<T> SpSc<T> {
	pub fn new(size:usize) -> SpSc<T> {
		let ring = SPSCNode::<T>::make_ring(size);
		SpSc{head:AtomicPtr::new(ring), tail:AtomicPtr::new(ring), size:size}
	}

	pub fn is_full(&self) -> bool {
		let head = self.head.load(Ordering::SeqCst);
		let tail = self.tail.load(Ordering::SeqCst);
		unsafe {
			head == tail && tail.as_ref().unwrap().0.load(Ordering::SeqCst) != ptr::null_mut()
		}
	}

    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::SeqCst);
        let tail = self.tail.load(Ordering::SeqCst);
        unsafe {
            head == tail && tail.as_ref().unwrap().0.load(Ordering::SeqCst) == ptr::null_mut()
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
	handle:Option<JoinHandle<()>>,
	switch:Switch,
	queue:TVal<SpSc<T>>,
    func:fn(*mut T)
}

impl<T: 'static> Clone for ExecUnit<T> {
    // Only clones queue size and function, does not copy the thread or current jobs
    fn clone(&self) -> Self {
        // new is hidden inside of clone to not clone the thread handle or queue
        ExecUnit::<T>::new(self.queue.size, self.func)
    }
}

impl<T: 'static> ExecUnit<T> {
    pub fn new(qsize:usize, func:fn(*mut T)) -> ExecUnit<T> {
        let queue = TVal::new(SpSc::new(qsize));
        let switch = Switch::new();
        switch.set(true);

        let tqueue = queue.clone();
        let tswitch = switch.clone();
        let handle = thread::spawn({move ||
	    		loop {
	    			if !tswitch.get() {
	    				// Finishes any remaining requests
	    				while let Some(ptr) = tqueue.pop() {
	    					func(ptr);
	    				}
	    				break;
	    			}
	    			while let Some(ptr) = tqueue.pop() {
	    				func(ptr);
	    			}
                    thread::park();
	    		}
	    	});
        ExecUnit{handle:Some(handle), switch:switch, queue:queue, func:func}
    }

    pub fn queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn give(&self, obj:T) -> bool {
    	let result = self.queue.push(alloc!(obj));
    	self.handle.as_ref().unwrap().thread().unpark();
    	return result;
    }

    pub fn give_ptr(&self, ptr:*mut T) -> bool {
    	let result = self.queue.push(ptr);
    	self.handle.as_ref().unwrap().thread().unpark();
    	return result;
    }

    pub fn stop(&mut self) {
    	self.switch.set(false);
        self.handle.as_ref().unwrap().thread().unpark();
    	self.handle.take().unwrap().join().unwrap();
    }
}

#[derive(Debug)]
pub struct ExecUnitGroup<T> {
    members:CircleList<ExecUnit<T>>,
    template:ExecUnit<T>
}

impl<T: 'static> ExecUnitGroup<T> {
    pub fn new(start_size:usize, qsize:usize, func:fn(*mut T)) -> ExecUnitGroup<T> {
        // Trick to form a clonable unit, but not start a thread for it
        let template = ExecUnit{handle:None, switch:Switch::new(), queue:TVal::new(SpSc{head:newptr!(), tail:newptr!(), size:qsize}), func:func};
        ExecUnitGroup{members:CircleList::new(&template, start_size), template:template}
    }

    pub fn increase_members(&self, amount:usize) {
        for _ in 0..amount {
            self.members.add(&self.template);
        }
    }

    pub fn assign_ptr(&self, ptr:*mut T) -> Option<usize> { // todo distribution
        for i in 0..self.members.len() {
            if self.members.next().give_ptr(ptr) {
                return Some(i);
            }
        }
        return None;
    }

    pub fn assign_retried(&self, ptr:*mut T, times:usize, pause:Duration) -> Option<usize> {
        for _ in 0..times {
            match self.assign_ptr(ptr) {
                Some(n) => return Some(n),
                None => thread::park_timeout(pause)
            }
        }
        return None;
    }

    pub fn stop_all(&mut self) {
        for _ in 0..self.members.len() {
            unsafe {
                self.members.next_ptr().as_mut().unwrap().get_mut().stop();
            }
        }
    }
}

// Intended to do gradual backoff for non blocking operations
// should never be used when holding a shared resource
#[derive(Debug, Clone)]
pub struct Parker {
    min_time:u64,
    max_time:u64,
    segment:u64,
    cur_time:u64
}

impl Parker {
    pub fn new(min_time:u64, max_time:u64, segment:u64) -> Parker {
        Parker{min_time:min_time, max_time:max_time, segment:segment, cur_time:min_time}
    }

    pub fn do_park(&mut self, result:bool) {
        if result {
            self.cur_time = self.min_time;
        } else if self.cur_time <= self.max_time {
            self.cur_time += self.segment;

        }
        thread::park_timeout(Duration::from_millis(self.cur_time));
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn parker_works() {
        let mut p = Parker::new(10, 100, 10);
        assert_eq!(p.cur_time, p.min_time);
        p.do_park(false);
        assert_eq!(p.cur_time, p.min_time + p.segment);
        p.do_park(true);
        assert_eq!(p.cur_time, p.min_time);
    }

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
    fn spsc_is_empty_works() {
        let qsize = 3;
        let queue = SpSc::<TestType>::new(qsize);
        assert!(queue.is_empty());
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

    fn sample_exec_func(obj:*mut u32) {
    	unsafe { *obj += 1; }
    }

    #[test]
    fn execunit_empty_works() {
    	let mut eunit = ExecUnit::new(5, sample_exec_func);
    	eunit.stop();
    }

    #[test]
    fn execunit_give_ptr_works() {
    	let mut eunit = ExecUnit::new(5, sample_exec_func);
    	let num = alloc!(5);
    	assert!(eunit.give_ptr(num));
    	eunit.stop();
    	unsafe {
    		assert_eq!(*num, 6);
    	}
    	free!(num);
    }

    #[test]
    fn execunit_before_stop_works() {
        let mut eunit = ExecUnit::new(5, sample_exec_func);
        let num = alloc!(5);
        assert!(eunit.give_ptr(num));
        while !eunit.queue_empty() {
            thread::yield_now();
        }
        unsafe {
            assert_eq!(*num, 6);
        }
        eunit.stop();
        free!(num);
    }

    #[test]
    fn execgroup_assign_ptr_works() {
        let mut egroup = ExecUnitGroup::new(3, 3, sample_exec_func);
        let num = alloc!(30);
        match egroup.assign_ptr(num) {
            Some(n) => assert_eq!(n, 0),
            None => panic!("Expected group {:?} to have an empty queue", egroup)
        }
        egroup.members.next();
        egroup.members.next();
        let chosen_unit = egroup.members.next();
        while !chosen_unit.queue_empty() {
            thread::yield_now();
        }
        unsafe {
            assert_eq!(*num, 31);
        }
        egroup.stop_all();
        free!(num);
    }

    #[test]
    fn execgroup_assign_retried_works() {
        let mut egroup = ExecUnitGroup::new(3, 3, sample_exec_func);
        let num = alloc!(30);
        let num2 = alloc!(15);
        match egroup.assign_retried(num, 5, Duration::from_millis(100)) {
            Some(n) => assert_eq!(n, 0),
            None => panic!("Expected group {:?} to have an empty queue", egroup)
        }
        match egroup.assign_retried(num2, 5, Duration::from_millis(100)) {
            Some(n) => assert_eq!(n, 0),
            None => panic!("Expected group {:?} to have an empty queue", egroup)
        }
        egroup.members.next();
        egroup.members.next();
        let chosen_unit = egroup.members.next();
        while !chosen_unit.queue_empty() {
            thread::yield_now();
        }
        let chosen_unit2 = egroup.members.next();
        while !chosen_unit2.queue_empty() {
            thread::yield_now();
        }
        unsafe {
            assert_eq!(*num, 31);
        }
        unsafe {
            assert_eq!(*num2, 16);
        }
        egroup.stop_all();
        free!(num);
        free!(num2);
    }
}
