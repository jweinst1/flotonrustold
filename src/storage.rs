use crate::ptrs::SharedPtr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;

pub struct StorageLink<T> {
	pub dlen:usize,
	pub data:Vec<SharedPtr<T>>,
	pub used:AtomicUsize,
	pub next:AtomicPtr<StorageLink<T>>
}

impl<T> StorageLink<T> {
	pub fn new(size:usize) -> *mut StorageLink<T> {
		let mut ptrs = vec![];
		for _ in 0..size {
			ptrs.push(SharedPtr::<T>::new(None));
		}
		Box::into_raw(
			Box::new(
				StorageLink{dlen:size, data:ptrs, used:AtomicUsize::new(0), next:AtomicPtr::new(ptr::null_mut())}
				)
			)
	}
}

pub struct StorageList<T> {
	pub head:AtomicPtr<StorageLink<T>>,
	pub link_size:AtomicUsize
}

static STORAGE_LIST_DEFAULT_LSIZE:usize = 10;

impl<T> Drop for StorageList<T> {
	// not thread safe, this should not be dropped in a multi-threaded context
    fn drop(&mut self) {
    	let mut cur_head = self.head.load(Ordering::SeqCst);
    	loop {
    		if cur_head == ptr::null_mut() {
    			break;
    		}
    		let manage = unsafe{ Box::from_raw(cur_head) };
    		cur_head = (*manage).next.load(Ordering::SeqCst);
    		drop(manage);
    	}
    }
}

impl<T> StorageList<T> {
	pub fn new(size:Option<usize>) -> StorageList<T> {
		match size {
			Some(s) => StorageList{head:AtomicPtr::new(StorageLink::new(s)), link_size:AtomicUsize::new(s)},
			None => StorageList{head:AtomicPtr::new(StorageLink::new(STORAGE_LIST_DEFAULT_LSIZE)), 
				                link_size:AtomicUsize::new(STORAGE_LIST_DEFAULT_LSIZE)}
		}
	}

	pub fn insert(&self, value:Option<T>) -> bool {
		let cur_head = self.head.load(Ordering::SeqCst);
		let cur_ref = unsafe { cur_head.as_ref().unwrap() };
		let place = cur_ref.used.fetch_add(1, Ordering::SeqCst);
		if place > cur_ref.dlen {
			cur_ref.used.fetch_sub(1, Ordering::SeqCst);
			// todo planning strategy
			// count towards a "miss"
			return false;
		} else if place == cur_ref.dlen {
			// new link coming
			let new_link = StorageLink::new(self.link_size.load(Ordering::SeqCst));
			let new_ref = unsafe { new_link.as_ref().unwrap() };
			let new_place = new_ref.used.fetch_add(1, Ordering::SeqCst);
			new_ref.data[new_place].reset(value, 1);
			new_ref.next.store(cur_head, Ordering::SeqCst);
			match self.head.compare_exchange(cur_head, new_link, Ordering::SeqCst, Ordering::SeqCst) {
				Ok(_) => { return true; },
				// This should never fail because only one caller gets place == dlen
				Err(p) => { panic!("Shouldn't get here! got pointer {:?}", p); }
			}
		} else {
			cur_ref.data[place].reset(value, 1);
			return true;
		}

	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_list_lsize_matches() {
    	let sl =  StorageList::<i32>::new(Some(5));
    	unsafe {
    		assert!(sl.link_size.load(Ordering::SeqCst) == sl.head.load(Ordering::SeqCst).as_ref().unwrap().dlen);
    	}
    }

    #[test]
    fn insert_works() {
    	let sl =  StorageList::<i32>::new(Some(5));
    	assert!(sl.insert(Some(3)));
    	unsafe {
    		let inserted = sl.head.load(Ordering::SeqCst).as_ref().unwrap().data[0].clone();
    		assert!(inserted.valid());
    		assert!(inserted.count().unwrap() == 2);
    		assert!(*inserted.get() == 3);
    	}
    }

    #[test]
    fn insert_used_is_correct() {
    	let sl =  StorageList::<i32>::new(Some(5));
    	assert!(sl.insert(Some(3)));
    	assert!(sl.insert(Some(3)));
    	unsafe {
    		assert!(sl.head.load(Ordering::SeqCst).as_ref().unwrap().used.load(Ordering::SeqCst) == 2);
    	}
    }

    #[test]
    fn insert_beyond_makes_new_link() {
    	let sl = StorageList::<i32>::new(Some(3));
    	assert!(sl.insert(Some(1)));
    	assert!(sl.insert(Some(1)));
    	assert!(sl.insert(Some(1)));
    	unsafe {
    		// any of the three inserts should all be ok
    		let inserted1 = sl.head.load(Ordering::SeqCst).as_ref().unwrap().data[0].clone();
    		let inserted2 = sl.head.load(Ordering::SeqCst).as_ref().unwrap().data[1].clone();
    		let inserted3 = sl.head.load(Ordering::SeqCst).as_ref().unwrap().data[2].clone();

    		assert!(inserted1.valid());
    		assert!(inserted1.count().unwrap() == 2);
    		assert!(*inserted1.get() == 1);

    		assert!(inserted2.valid());
    		assert!(inserted2.count().unwrap() == 2);
    		assert!(*inserted2.get() == 1);

    		assert!(inserted3.valid());
    		assert!(inserted3.count().unwrap() == 2);
    		assert!(*inserted3.get() == 1);
    	}
    	assert!(sl.insert(Some(4)));
    	unsafe {
    		let inserted4 = sl.head.load(Ordering::SeqCst).as_ref().unwrap().data[0].clone();
    		assert!(inserted4.valid());
    		assert!(inserted4.count().unwrap() == 2);
    		assert!(*inserted4.get() == 4);

    		match sl.head.load(Ordering::SeqCst).as_ref().unwrap().next.load(Ordering::SeqCst).as_ref() {
    			Some(n) => {
    				// make sure previous link is preserved
    				let inserted_prev1 = n.data[0].clone();
		    		assert!(inserted_prev1.valid());
		    		assert!(inserted_prev1.count().unwrap() == 2);
		    		assert!(*inserted_prev1.get() == 1);
    			},
    			None => { panic!("No valid pointer to next list elements!!"); }
    		}
    	}
    }

    #[test]
    fn full_link_insert_returns_false() {
    	let sl = StorageList::<i32>::new(Some(3));
    	unsafe {
    		sl.head.load(Ordering::SeqCst).as_ref().unwrap().used.store(4, Ordering::SeqCst);
    	}
    	assert!(!sl.insert(Some(5)));
    }
}