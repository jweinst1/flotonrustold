use std::sync::atomic::AtomicPtr;
use std::ptr;

macro_rules! ptref {
	($obj:expr) => { unsafe { $obj.as_ref().unwrap() } }
}

macro_rules! isnull {
	($obj:expr) => { $obj == ptr::null_mut() }
}

macro_rules! newptr {
	() => { AtomicPtr::new(ptr::null_mut()) }
}

macro_rules! alloc {
    ($obj:expr) => {
        Box::into_raw(Box::new($obj))
    };
}

macro_rules! free {
    ($obj:expr) => {
        unsafe { drop(Box::from_raw($obj)); }
    };
}