use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::{thread, ptr};
use std::time::{Duration, Instant};
use std::convert::TryFrom;

static MONOTONIC_EPOCH:AtomicPtr<Instant> = AtomicPtr::new(ptr::null_mut());

// only call once
pub fn set_epoch() {
	MONOTONIC_EPOCH.store(Box::into_raw(Box::new(Instant::now())), Ordering::SeqCst);
}

pub fn destroy_epoch() {
	let swapped_out = MONOTONIC_EPOCH.swap(ptr::null_mut(), Ordering::SeqCst);
	unsafe { drop(Box::from_raw(swapped_out)); }
}

pub fn get_time() -> u64 {
	unsafe {
		match MONOTONIC_EPOCH.load(Ordering::SeqCst).as_ref() {
			// todo, configure precision
			Some(r) => match u64::try_from(r.elapsed().as_nanos()) {
				Ok(v) => v,
				Err(e) => panic!("Could not convert monotonic tick to u64, err: {:?}", e)
			},
			None => panic!("MONOTONIC_EPOCH was loaded but not initialized!")
		}
	}
}

pub struct TimePtr<T>(pub *mut T/*object*/, pub u64 /*time*/, pub u8 /*thread id*/);

impl<T> TimePtr<T> {
    pub fn make(val:T, tid:u8) -> *mut TimePtr<T> {
        Box::into_raw(Box::new(TimePtr(Box::into_raw(Box::new(val)), get_time(), tid)))
    }

    pub fn owned_by(ptr:*mut TimePtr<T>, tid:u8) -> bool {
        unsafe {
            match ptr.as_ref() {
                Some(r) => r.2 == tid,
                None => false
            }
        }
    }

    pub fn get_time(ptr:*mut TimePtr<T>) -> u64 {
        unsafe {
            match ptr.as_ref() {
                Some(r) => r.1,
                None => panic!("Attempted to `get_time` of nullptr")
            }
        }
    }

    fn lt(lfs:*mut TimePtr<T>, rfs:*mut TimePtr<T>) -> bool {
         unsafe {
              match lfs.as_ref() {
                   Some(rl) => match rfs.as_ref() {
                        Some(rr) => rl.1 < rr.1,
                        None => false
                   },
                   None => false
              }
         }
    }

    fn gt(lfs:*mut TimePtr<T>, rfs:*mut TimePtr<T>) -> bool {
         unsafe {
              match lfs.as_ref() {
                   Some(rl) => match rfs.as_ref() {
                        Some(rr) => rl.1 > rr.1,
                        None => false
                   },
                   None => false
              }
         }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};

    #[test]
    fn get_time_works() {
    	set_epoch();
    	let t1 = get_time();
    	let t2 = get_time();
    	assert!(t1 < t2);
    	destroy_epoch();
    }
}