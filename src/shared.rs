use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::{thread, time, ptr};

struct SharedCount<T>(*mut T, AtomicUsize);

impl<T> SharedCount<T> {
    fn make(val:T) -> *mut SharedCount<T> {
        Box::into_raw(Box::new(SharedCount(Box::into_raw(Box::new(val)), AtomicUsize::new(1))))
    }

    fn dec_count(ptr:*mut SharedCount<T>) {
    	unsafe {
    		match ptr.as_ref() {
    			Some(r) => {
    				if r.1.fetch_sub(1, Ordering::SeqCst) == 1 {
    					drop(Box::from_raw(r.0));
    					drop(Box::from_raw(ptr));
    				}
    			},
    			None => ()
    		}
    	}
    }
}


// state is always read left to right
#[derive(PartialEq)]
#[derive(Debug)]
enum AccessState {
	RrCc = 0,
	RcRc = 1,
	CcRr = 2,
	CrCr = 3
}

struct AccessStateCount(AccessState, u64);

struct AccessCounter(AtomicU64);

impl AccessCounter {
	fn new(state:AccessState) -> AccessCounter {
		AccessCounter(AtomicU64::new(state as u64))
	}

	fn count(&self) -> u64 {
		self.0.load(Ordering::SeqCst) >> 2
	}

	fn state(&self) -> AccessState {
		match self.0.load(Ordering::SeqCst) & 0b11 {
			0 => AccessState::RrCc,
			1 => AccessState::RcRc,
			2 => AccessState::CcRr,
			3 => AccessState::CrCr
		}
	}

	fn state_and_count(&self) -> AccessStateCount {
		let value = self.0.load(Ordering:SeqCst);
		let state = match value & 0b11 {
			0 => AccessState::RrCc,
			1 => AccessState::RcRc,
			2 => AccessState::CcRr,
			3 => AccessState::CrCr
		};
		return AccessStateCount(state, value >> 2)
	}

	// Always adds in increment of 0b100, to never touch 2bit flag
	fn inc(&self) -> u64 {
		self.0.fetch_add(0b100, Ordering::SeqCst)
	}
	// Always subs in decrement of 0b100, to never touch 2bit flag
	fn dec(&self) -> u64 {
		self.0.fetch_sub(0b100, Ordering::SeqCst)
	}

	// todo - swap to next
}

struct CountedPtr<T>(AtomicPtr<SharedCount<T>>, AtomicU32, AtomicU32);

impl<T> CountedPtr<T> {
	fn new(ptr:*mut SharedCount<T>) -> CountedPtr<T> {
		CountedPtr(AtomicPtr::new(ptr), AtomicU32::new(0), AtomicU32::new(0))
	}

	fn duplicate(&self) -> *mut SharedCount<T> {
		self.1.fetch_add(1, Ordering::SeqCst);
		if self.2.load(Ordering::SeqCst) > 0 {
			// reset is in progress
			self.1.fetch_sub(1, Ordering::SeqCst);
			return ptr::null_mut();
		}
		let ptr = self.0.load(Ordering::SeqCst);
		unsafe {
			match ptr.as_ref() {
				Some(r) => {
					r.1.fetch_add(1, Ordering::SeqCst);
				},
				None => {
					self.1.fetch_sub(1, Ordering::SeqCst);
					return ptr::null_mut();
				}
			}
	    }

		self.1.fetch_sub(1, Ordering::SeqCst);
		return ptr;
	}

	fn reset(&self, ptr:*mut SharedCount<T>) -> bool {
		self.2.fetch_add(1, Ordering::SeqCst);
		if self.1.load(Ordering::SeqCst) > 0 {
			// clone is in progress
			self.2.fetch_sub(1, Ordering::SeqCst);
			return false;
		}
		let swapped_out = self.0.swap(ptr, Ordering::SeqCst);
		SharedCount::dec_count(swapped_out);
		return true;
	}
}

pub struct Shared<T> {
	bins:[CountedPtr<T>; 4],
	new_cnt:AccessCounter,
	old_cnt:AccessCounter
}

impl<T> Shared<T> {
	pub fn new() -> Shared<T> {
		Shared{bins:[CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut())], // 4th slot is initial
			  new_cnt:AccessCounter::new(AccessState::RrCc),
			  old_cnt:AccessCounter::new(AccessState::RrCc)
			     }
	}

	fn new_ptr(ptr:*mut SharedCount<T>) -> Shared<T> {
		Shared{bins:[CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr)], // 4th slot is initial
			  new_cnt:AccessCounter::new(AccessState::RrCc),
			  old_cnt:AccessCounter::new(AccessState::RrCc)
			     }	
	}

	pub fn clone(&self) -> Shared<T> {
		let key = self.new_cnt.inc();
		let state = key & 0b11;
		let count =  key >> 2;
		let mut cloned = ptr::null_mut();
		match state {
			AccessState::RrCc => {
				cloned = self.bins[3].duplicate();
				if cloned == ptr::null_mut() {
					cloned = self.bins[2].duplicate();
					assert!(cloned != ptr::null_mut());
				}

			},
			AccessState::RcRc => {},
			AccessState::CcRr => {},
			AccessState::CrCr => {}
		}
		if !AccessState::eq_u64(self.new_cnt.load(Ordering::SeqCst), state) {
			// the state has changed, decrement counter from previous generation
			self.old_cnt.fetch_sub(1, Ordering::SeqCst);
		} else {
			self.new_cnt.fetch_sub(1, Ordering::SeqCst);
		}
		return Shared::new_ptr(cloned);
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u64_works() {
    	let cnt:u64 = (2 << 62) + 3;
    	let state = AccessState::from_u64(cnt);
    	assert_eq!(state, AccessState::CcRr);
    }

    #[test]
    fn eq_u64_works() {
    	let cnt1:u64 = (2 << 62) + 3;
    	let cnt2:u64 = (2 << 62) + 56;
    	let cnt3:u64 = (1 << 62) + 3;
    	assert!(AccessState::eq_u64(cnt1, cnt2));
    	assert!(!AccessState::eq_u64(cnt2, cnt3));
    }

    #[test]
    fn count_u64_works() {
    	let cnt:u64 = (2 << 62) + 56;
    	assert!(AccessState::count_u64(cnt) == 56);
    }
}