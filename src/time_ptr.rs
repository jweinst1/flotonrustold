use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::{thread, ptr};
use std::time::{Duration, Instant};

static MONOTONIC_EPOCH:AtomicPtr<Instant> = AtomicPtr::new(ptr::null_mut());

struct TimePtr<T>(*mut T/*object*/, u64 /*time*/, u8 /*thread id*/);

impl<T> TimePtr<T> {
    fn make(val:T, tval:u64, tid:u8) -> *mut TimePtr<T> {
        Box::into_raw(Box::new(TimePtr(Box::into_raw(Box::new(val)), tval, tid)))
    }

    fn owned_by(ptr:*mut TimePtr<T>, tid:u8) -> bool {
        unsafe {
            match ptr.as_ref() {
                Some(r) => r.2 == tid,
                None => false
            }
        }
    }

    fn get_time(ptr:*mut TimePtr<T>) -> u64 {
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