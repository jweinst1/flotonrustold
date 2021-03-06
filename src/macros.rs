

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

#[macro_export]
macro_rules! alloc {
    ($obj:expr) => {
        Box::into_raw(Box::new($obj))
    };
}

#[macro_export]
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
    // Fires off a thread that runs the statement some number of times,
    // while also pausing for some amount of milliseconds between each call.
    ($dur:expr, $times:expr, $a:ident.$($b:tt)+) => {
        {
            let rptr = AtomicPtr::new(&mut $a);
            thread::spawn(move || {
                for _ in  0..($times) {
                     unsafe { rptr.load(Ordering::SeqCst).as_ref().unwrap().$($b)+; }
                     thread::park_timeout(Duration::from_millis($dur));
                }
            })
        }
    };

    // Fires off a thread that runs the statement some number of times
    ($times:expr, $a:ident.$($b:tt)+) => {
        {
            let rptr = AtomicPtr::new(&mut $a);
            thread::spawn(move || {
                for _ in  0..($times) {
                     unsafe { rptr.load(Ordering::SeqCst).as_ref().unwrap().$($b)+; }
                }
            })
        }
    };

    ($a:ident.$($b:tt)+) => {
        {
            let rptr = AtomicPtr::new(&mut $a);
            thread::spawn(move || { unsafe { rptr.load(Ordering::SeqCst).as_ref().unwrap().$($b)+; } })
        }
    };

    ($dur:expr, $times:expr, $($b:tt)+) => {
        {
            thread::spawn(move || {
                for _ in  0..($times) {
                     $($b)+;
                     thread::park_timeout(Duration::from_millis($dur));
                }
            })
        }
    };

    // This just moves a whole block as thread spawn
    ($code:block) => {
        {
            thread::spawn(move || $code)
        }
    };

    ($($b:tt)+) => {
        thread::spawn(||{ 
            $($b)+;
        })
    };
}