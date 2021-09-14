use std::sync::atomic::{AtomicI32, Ordering};
use std::fmt::Display;
use crate::constants::*;
use crate::datetime::DateTime;
use crate::traits::*;

pub const LOG_LEVEL_FATAL:i32 = 0;
pub const LOG_LEVEL_ERROR:i32 = 1;
pub const LOG_LEVEL_WARN:i32 = 2;
pub const LOG_LEVEL_INFO:i32 = 3;
pub const LOG_LEVEL_DEBUG:i32 = 4;
pub const LOG_LEVEL_TRACE:i32 = 5;

// By default, at least fatal level logs are logged
pub static GLOBAL_LOGGING_LEVEL:AtomicI32 = AtomicI32::new(LOG_LEVEL_FATAL);

#[inline(always)]
pub fn logging_make_new_date_time() -> DateTime {
	DateTime::new()
}

macro_rules! log_fatal {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_FATAL {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} FATAL {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

macro_rules! log_error {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_ERROR {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} ERROR {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

macro_rules! log_warn {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_WARN {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} WARN {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

macro_rules! log_info {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_INFO {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} INFO {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

macro_rules! log_debug {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_DEBUG {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} DEBUG {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

macro_rules! log_trace {
	($component:ident, $($b:tt)+) => {
		if GLOBAL_LOGGING_LEVEL.load(Ordering::Relaxed) >= LOG_LEVEL_TRACE {
			let mut dt = logging_make_new_date_time();
			dt.set_to_now().expect("Could not set DateTime from gmtime");
			println!("{} TRACE {} - {}", dt, stringify!($component), format!($($b)+));
		}
	};
}

// only use from unit tests
// the real level should only be changed at global, not scope level
pub fn logging_test_set(level:i32) {
	GLOBAL_LOGGING_LEVEL.store(level, Ordering::Relaxed);
}
