

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