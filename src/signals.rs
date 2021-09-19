extern crate libc;
use std::sync::atomic::Ordering;
use crate::logging::*;

pub fn register_int_handler(handle:fn(i32)) {
	unsafe { libc::signal(libc::SIGINT, handle as usize); }
	log_debug!(Signal, "Signal handler: {:?} registered for sig int", handle);
}

pub fn register_term_handler(handle:fn(i32)) {
	unsafe { libc::signal(libc::SIGTERM, handle as usize); }
	log_debug!(Signal, "Signal handler: {:?} registered for sig term", handle);
}

