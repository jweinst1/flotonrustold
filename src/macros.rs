use std::sync::atomic::AtomicPtr;
use std::{ptr, thread};

macro_rules! ptref {
	($obj:expr) => { unsafe { $obj.as_ref().unwrap() } }
}

macro_rules! isnull {
	($obj:expr) => { $obj == ptr::null_mut() }
}

macro_rules! nonull {
	($obj:expr) => { $obj != ptr::null_mut() }
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

/**
 * This is used to run an <var>.<method>(<args ...>) expression in another thread
 * and return the handle. This is primarily used for unit tests.
 */
macro_rules! thcall {
    ($a:ident.$($b:tt)+) => {
        thread::spawn(||{ 
            $a.$($b)+;
            $a
        })
    };

    ($($b:tt)+) => {
        thread::spawn(||{ 
            $($b)+;
        })
    }
}