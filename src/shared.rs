use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::{thread, time, ptr};

struct SharedCount<T>(*mut T, AtomicUsize);

impl<T> SharedCount<T> {
    fn make(val:T) -> *mut SharedCount<T> {
        Box::into_raw(Box::new(SharedCount(Box::into_raw(Box::new(val)), AtomicUsize::new(1))))
    }
}


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

	fn eq_u64(lfs:u64, rfs:u64) -> bool {
		// todo, make inline ?
		AccessState::from_u64(lfs) == AccessState::from_u64(rfs)
	}
}

struct AccessCounted<T>(AtomicPtr<SharedCount<T>>, AtomicU32, AtomicU32);

impl<T> AccessCounted<T> {
	fn new() -> AccessCounted<T> {
		AccessCounted(AtomicPtr::new(ptr::null_mut()), AtomicU32::new(0), AtomicU32::new(0))
	}
}

pub struct Shared<T> {
	bins:[AccessCounted<T>; 4],
	//ctrl:AccessControl
}

impl<T> Shared<T> {
	pub fn new() -> Shared<T> {
		Shared{bins:[AccessCounted::new(),
			         AccessCounted::new(),
			         AccessCounted::new(),
			         AccessCounted::new()]
			    //ctrl:AccessControl::new()
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
}