use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
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
	fn from_u64(val:u64) -> AccessState {
		match val & 0b11 {
			0 => AccessState::RrCc,
			1 => AccessState::RcRc,
			2 => AccessState::CcRr,
			3 => AccessState::CrCr,
			_ => panic!("Unexpected value {:?}", val & 0b11)
		}
	}
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
		AccessState::from_u64(self.0.load(Ordering::SeqCst))
	}

	fn state_and_count(&self) -> AccessStateCount {
		let value = self.0.load(Ordering::SeqCst);
		AccessStateCount(AccessState::from_u64(value), value >> 2)
	}

	// Always adds in increment of 0b100, to never touch 2bit flag
	fn inc(&self) -> u64 {
		self.0.fetch_add(0b100, Ordering::SeqCst)
	}

	fn inc_by(&self, amount:u64) -> u64 {
		self.0.fetch_add(amount >> 2, Ordering::SeqCst)
	}

	fn inc_and_get_state(&self) -> AccessState {
		AccessState::from_u64(self.0.fetch_add(0b100, Ordering::SeqCst))
	}
	// Always subs in decrement of 0b100, to never touch 2bit flag
	fn dec(&self) -> u64 {
		self.0.fetch_sub(0b100, Ordering::SeqCst)
	}

	fn dec_and_get_state(&self) -> AccessState {
		AccessState::from_u64(self.0.fetch_sub(0b100, Ordering::SeqCst))
	}
	// This must be called by a single thread at a time
	fn swap_to_next_state(&self) -> AccessStateCount {
		let swapped_out = match self.state() {
			AccessState::RrCc => {
				self.0.swap(AccessState::RcRc as u64, Ordering::SeqCst)
			},
			AccessState::RcRc => {
				self.0.swap(AccessState::CcRr as u64, Ordering::SeqCst)
			},
			AccessState::CcRr => {
				self.0.swap(AccessState::CrCr as u64, Ordering::SeqCst)
			},
			AccessState::CrCr => {
				self.0.swap(AccessState::RrCc as u64, Ordering::SeqCst)
			}
		};
		AccessStateCount(AccessState::from_u64(swapped_out), swapped_out >> 2)
	}
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
		self.2.fetch_sub(1, Ordering::SeqCst);
		return true;
	}
}

impl<T> Drop for CountedPtr<T> {
    fn drop(&mut self) {
        SharedCount::dec_count(self.0.load(Ordering::SeqCst))
    }
}

pub struct Shared<T> {
	bins:[CountedPtr<T>; 4],
	new_cnt:AccessCounter,
	old_cnt:AccessCounter,
	gen_key:AtomicBool,
	gen_ready:AtomicBool
}

impl<T> Shared<T> {
	pub fn new() -> Shared<T> {
		Shared{bins:[CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut())], // 4th slot is initial
			  new_cnt:AccessCounter::new(AccessState::RrCc),
			  old_cnt:AccessCounter::new(AccessState::RrCc),
			  gen_key:AtomicBool::new(true),
			  gen_ready:AtomicBool::new(false)
			     }
	}

	fn new_ptr(ptr:*mut SharedCount<T>) -> Shared<T> {
		Shared{bins:[CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr::null_mut()),
			         CountedPtr::new(ptr)], // 4th slot is initial
			  new_cnt:AccessCounter::new(AccessState::RrCc),
			  old_cnt:AccessCounter::new(AccessState::RrCc),
			  gen_key:AtomicBool::new(true),
			  gen_ready:AtomicBool::new(false)
			     }	
	}

	fn grab_key(&self) -> bool {
		self.gen_key.swap(false, Ordering::SeqCst)
	}

	fn reset_sc(&self, ptr:*mut SharedCount<T>) {
		let state = self.new_cnt.inc_and_get_state();
		match state {
			AccessState::RrCc => {
				if !self.bins[1].reset(ptr) {
					assert!(self.bins[0].reset(ptr));
				}
			},
			AccessState::RcRc => {
				if !self.bins[0].reset(ptr) {
					assert!(self.bins[2].reset(ptr));
				}				
			},
			AccessState::CcRr => {
				if !self.bins[2].reset(ptr) {
					assert!(self.bins[3].reset(ptr));
				}
			},
			AccessState::CrCr => {
				if !self.bins[3].reset(ptr) {
					assert!(self.bins[1].reset(ptr));
				}
			}
		}
		let state2 = self.new_cnt.dec_and_get_state();
		if state != state2 {
			// new to correct the count
			self.new_cnt.inc();
			// This attendance count belongs to the old generation
			self.old_cnt.dec();
		}
		// Attempt next generation advancement
		if self.old_cnt.count() == 0 {
			if self.grab_key() {
				let state_count = self.new_cnt.swap_to_next_state();
				self.old_cnt.inc_by(state_count.1);
				// put key back when finished
				self.gen_key.store(true, Ordering::SeqCst);
			}
		} else {
			// a clone will take over this duty
		    self.gen_ready.store(true, Ordering::SeqCst);
		}
	}

	pub fn reset(&self, val:Option<T>) {
		match val {
			Some(v) => self.reset_sc(SharedCount::make(v)),
			None => self.reset_sc(ptr::null_mut())
		}
	}

	fn clone_sc(&self) -> *mut SharedCount<T> {
		let state = self.new_cnt.inc_and_get_state();
		let mut cloned = ptr::null_mut();
		match state {
			AccessState::RrCc => {
				cloned = self.bins[3].duplicate();
				if cloned == ptr::null_mut() {
					cloned = self.bins[2].duplicate();
					assert!(cloned != ptr::null_mut());
				}
			},
			AccessState::RcRc => {
				cloned = self.bins[1].duplicate();
				if cloned == ptr::null_mut() {
					cloned = self.bins[3].duplicate();
					assert!(cloned != ptr::null_mut());
				}
			},
			AccessState::CcRr => {
				cloned = self.bins[0].duplicate();
				if cloned == ptr::null_mut() {
					cloned = self.bins[1].duplicate();
					assert!(cloned != ptr::null_mut());
				}				
			},
			AccessState::CrCr => {
				cloned = self.bins[2].duplicate();
				if cloned == ptr::null_mut() {
					cloned = self.bins[0].duplicate();
					assert!(cloned != ptr::null_mut());
				}
			}
		}
		let state2 = self.new_cnt.dec_and_get_state();
		if state != state2 {
			// new to correct the count
			self.new_cnt.inc();
			// This attendance count belongs to the old generation
			self.old_cnt.dec();
		}

		if self.gen_ready.swap(false, Ordering::SeqCst) {
			if self.grab_key() {
				let state_count = self.new_cnt.swap_to_next_state();
				self.old_cnt.inc_by(state_count.1);
				// put key back when finished
				self.gen_key.store(true, Ordering::SeqCst);
			}
			// no need to put back key since may not be time to advance yet
		}
		return cloned;
	}

	pub fn clone(&self) -> Shared<T> {
		Shared::new_ptr(self.clone_sc())
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    	
    }
}