use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
use std::ptr;

// Works to cache non-owning pointers via a wait-free stack
struct CStack<T> {
	dlen:i64,
	data:Vec<AtomicPtr<T>>,
	head:AtomicI64
}

impl<T> CStack<T> {
    fn new(size:i64) -> CStack<T> {
        let mut ptrs = vec![];
        for _ in 0..size {
            ptrs.push(AtomicPtr::new(ptr::null_mut()));
        }
        CStack{dlen:size, data:ptrs, head:AtomicI64::new(0)}
    }

    fn push(&self, ptr:*mut T) -> bool { // to do replace with reason
        let place = self.head.fetch_add(1, Ordering::SeqCst);
        if place < 0 {
            // conflict with a pop,
            return false;
        } else if place >= self.dlen {
            self.head.fetch_sub(1, Ordering::SeqCst);
            return false;
        } else {
            match self.data[place as usize].compare_exchange(ptr::null_mut(), ptr, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => { return true; },
                Err(_) => { return false; }
            }
        }
    }
    
    fn pop(&self) -> Option<*mut T > {
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