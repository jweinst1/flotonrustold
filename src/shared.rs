use std::sync::atomic::{AtomicPtr, AtomicBool, AtomicUsize, AtomicU32, AtomicU64, Ordering};
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

impl AccessState {
	fn from_u64(value:u64) -> AccessState {
		match value >> 62 {
			0 => AccessState::RrCc,
			1 => AccessState::RcRc,
			2 => AccessState::CcRr,
			3 => AccessState::CrCr,
			_ => panic!("Unhandled access state {:?}", value)
		}
	}

	fn initial() -> u64 {
		AccessState::RrCc as u64
	}

	fn eq_u64(lfs:u64, rfs:u64) -> bool {
		// todo, make inline ?
		AccessState::from_u64(lfs) == AccessState::from_u64(rfs)
	}

	fn count_u64(value:u64) -> u64 {
		value & !(3 << 62)
	}
}

struct AccessCounted<T>(AtomicPtr<SharedCount<T>>, AtomicU32, AtomicU32);

impl<T> AccessCounted<T> {
	fn new(ptr:*mut SharedCount<T>) -> AccessCounted<T> {
		AccessCounted(AtomicPtr::new(ptr), AtomicU32::new(0), AtomicU32::new(0))
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
	bins:[AccessCounted<T>; 4],
	new_cnt:AtomicU64,
	old_cnt:AtomicU64
}

impl<T> Shared<T> {
	pub fn new() -> Shared<T> {
		Shared{bins:[AccessCounted::new(ptr::null_mut()),
			         AccessCounted::new(ptr::null_mut()),
			         AccessCounted::new(ptr::null_mut()),
			         AccessCounted::new(ptr::null_mut())], // 4th slot is initial
			  new_cnt:AtomicU64::new(AccessState::initial()),
			  old_cnt:AtomicU64::new(0)
			     }
	}

	pub fn clone(&self) -> Shared<T> {
		let key = self.new_cnt.fetch_add(1, Ordering::SeqCst);
		let state = AccessState::from_u64(key);
		let count = AccessState::count_u64(key);
		match state {
			AccessState::RrCc => {
				// 3 for clone
			},
			AccessState::RcRc => {},
			AccessState::CcRr => {},
			AccessState::CrCr => {}
		}
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