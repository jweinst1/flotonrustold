use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
use std::ptr;

// Works to cache non-owning pointers via a wait-free stack
pub struct CStack<T> {
	pub dlen:i64,
	pub data:Vec<AtomicPtr<T>>,
	pub head:AtomicI64,
	pub next:AtomicPtr<CStack<T>>
}

impl<T> CStack<T> {
    pub fn new(size:i64) -> CStack<T> {
        let mut ptrs = vec![];
        for _ in 0..size {
            ptrs.push(AtomicPtr::new(ptr::null_mut()));
        }
        CStack{dlen:size, data:ptrs, head:AtomicI64::new(0), next:AtomicPtr::new(ptr::null_mut())}
    }

    fn push(&self, ptr:*mut T) -> bool { // to do replace with reason
        let place = self.head.fetch_add(1, Ordering::SeqCst);
        if place < 0 {
            // conflict with a pop,
            return false;
        } else if place == self.dlen {
        	// time for new node
        	let new_stack = CStack::new(self.dlen);
        	// This does not need to go through push() since this stack is still thread local
        	let new_stack_place = new_stack.head.fetch_add(1, Ordering::SeqCst);
        	new_stack.data[new_stack_place as usize].store(ptr, Ordering::SeqCst);
        	let new_stack_ptr = Box::into_raw(Box::new(new_stack));
        	match self.next.compare_exchange(ptr::null_mut(), new_stack_ptr, Ordering::SeqCst, Ordering::SeqCst) {
        		Ok(_) => { return true; },
        		Err(_) => {
        			self.head.fetch_sub(1, Ordering::SeqCst);
        			// todo monitoring and statistics
        		    unsafe { drop(Box::from_raw(new_stack_ptr)); }
        		    // todo recursive limited calls
        			return false;
        		}
        	}
        } else if place > self.dlen {
            self.head.fetch_sub(1, Ordering::SeqCst);
            return false;
        } else {
            match self.data[place as usize].compare_exchange(ptr::null_mut(), ptr, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => { return true; },
                Err(_) => { return false; }
            }
        }
    }
    
    fn pop(&self) -> Option<*mut T > { // todo reason 
        let place = self.head.fetch_sub(1, Ordering::SeqCst) - 1;
        if place < 0 {
            self.head.fetch_add(1, Ordering::SeqCst);
            return None;
        } else if place >= self.dlen {
            // will be corrected on push side
            return None;
        } else {
            let cur_p = self.data[place as usize].load(Ordering::SeqCst);
            if cur_p == ptr::null_mut() {
                return None;
            }
            match self.data[place as usize].compare_exchange(cur_p, ptr::null_mut(), Ordering::SeqCst, Ordering::SeqCst) {
                Ok(p) => { return Some(p); },
                Err(_) => { return None; }
            }
        }
    }
}

// This is constructed in forward fashion
// stack size could change based on policy
pub struct CStackLine<T> {
	pub front:CStack<T>,
}

impl<T> Drop for CStackLine<T> {
	// not thread safe, this should not be dropped in a multi-threaded context
    fn drop(&mut self) {
    	// first cstack never needs to be manually dropped
    	let mut cur_front = self.front.next.load(Ordering::SeqCst);
    	loop {
    		if cur_front == ptr::null_mut() {
    			break;
    		}
    		let manage = unsafe{ Box::from_raw(cur_front) };
    		cur_front = (*manage).next.load(Ordering::SeqCst);
    		drop(manage);
    	}
    }
}

impl<T> CStackLine<T> {
	pub fn new(size:i64) -> CStackLine<T> {
		CStackLine{front:CStack::new(size)}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cstack_new_length() {
    	let c1 = CStack::<i32>::new(10);
    	assert!(c1.data.len() == 10);
    	assert!(c1.dlen == 10);
    }

    #[test]
    fn cstack_base_push_pop() {
    	let c1 = CStack::new(10);
    	assert!(c1.push(Box::into_raw(Box::new(5))));
    	match c1.pop() {
    		Some(p) => unsafe { assert!(*p == 5); drop(Box::from_raw(p)); },
    		None => { panic!("Expected a pointer to be popped!!"); }
    	}
    }
}