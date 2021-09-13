extern crate libc;
use std::ptr;

pub fn unix_time() -> u64 {
	unsafe { libc::time(ptr::null_mut()) as u64 }
}