use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU8, Ordering};
use std::{thread, time, ptr};

struct SharedCount<T>(*mut T, AtomicUsize);

impl<T> SharedCount<T> {
    fn make(val:T) -> *mut SharedCount<T> {
        Box::into_raw(Box::new(SharedCount(Box::into_raw(Box::new(val)), AtomicUsize::new(1))))
    }
}

#[repr(u8)]
#[derive(PartialEq)]
enum AccessState {
	RrCc = 0b00110000,
	RcRc = 0b01010000,
	CcRr = 0b11000000,
	CrCr = 0b10100000
}

impl AccessState {
	fn from_u8(value:u8) -> AccessState {
		match value & 0b11110000 {
			0b00110000 => AccessState::RrCc,
			0b01010000 => AccessState::RcRc,
			0b11000000 => AccessState::CcRr,
			0b10100000 => AccessState::CrCr,
			_ => panic!("Unhandled access state {:?}", value)
		}
	}

	fn next_u8(value:u8) -> AccessState {
		match value & 0b11110000 {
			0b00110000 => AccessState::RcRc,
			0b01010000 => AccessState::CcRr,
			0b11000000 => AccessState::CrCr,
			0b10100000 => AccessState::RrCc,
			_ => panic!("Unhandled access state {:?}", value)
		}
	}
}

struct AccessControl(AtomicU8);

impl AccessControl {
	fn new() -> AccessControl {
		AccessControl(AtomicU8::new(AccessState::RrCc as u8))
	}

	fn ungate(&self, pos:usize) {
		self.0.fetch_or(1 << pos, Ordering::SeqCst);
	}

	fn value(&self) -> u8 {
		self.0.load(Ordering::SeqCst)
	}

	fn state(&self) -> AccessState {
		AccessState::from_u8(self.0.load(Ordering::SeqCst))
	}

	fn is_fully_open(&self) -> bool {
		(self.0.load(Ordering::SeqCst) & 0b1111) == 0b1111
	}
}

struct AccessCounted<T>(AtomicPtr<SharedCount<T>>, AtomicUsize, AtomicUsize);

impl<T> AccessCounted<T> {
	fn new() -> AccessCounted<T> {
		AccessCounted(AtomicPtr::new(ptr::null_mut()), AtomicUsize::new(0), AtomicUsize::new(0))
	}
}

pub struct Shared<T> {
	bins:[AccessCounted<T>; 4],
	ctrl:AccessControl
}

impl<T> Shared<T> {
	pub fn new() -> Shared<T> {
		Shared{bins:[AccessCounted::new(),
			         AccessCounted::new(),
			         AccessCounted::new(),
			         AccessCounted::new()],
			    ctrl:AccessControl::new()
			     }
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ungate_works() {
    	let control = AccessControl::new();
    	control.ungate(0);
    	assert!(control.value() == 0b00110001);
    }

    #[test]
    fn next_u8_works() {
    	assert!(AccessState::next_u8(AccessState::RrCc as u8) == AccessState::RcRc);
    	assert!(AccessState::next_u8(AccessState::RcRc as u8) == AccessState::CcRr);
    	assert!(AccessState::next_u8(AccessState::CcRr as u8) == AccessState::CrCr);
    	assert!(AccessState::next_u8(AccessState::CrCr as u8) == AccessState::RrCc);
    }
}