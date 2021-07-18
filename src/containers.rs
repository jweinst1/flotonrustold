use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use crate::shared::*;

#[derive(Debug)]
pub enum Container<T> {
	Val(Shared<T>),
	List(Vec<Shared<Container<T>>>)
}

impl<T: Debug> Container<T> {
	pub fn new_list(size:usize) -> Container<T> {
		let mut fields = vec![];
		fields.reserve(size);
		for _ in 0..size {
			fields.push(Shared::<Container<T>>::new());
		}
		Container::List(fields)
	}

	pub fn set_list(&self, pos:usize, val:T, tid:usize) {
		match self {
			Container::Val(v) => panic!("Expected List, got Val({:?})", v),
			Container::List(l) => l[pos].write(TimePtr::make(Container::Val(Shared::new_val(val))), tid)
		}
	}

	pub fn set_val(&self, val:T, tid:usize) {
		match self {
			Container::Val(v) => v.write(TimePtr::make(val), tid),
			Container::List(l) => panic!("Expected Val, got List({:?})", l)
		}
	}

	pub fn get_list(&self, pos:usize, tid:usize) -> Option<&Container<T>> {
		match self {
			Container::List(l) => unsafe {
				match l[pos].read(tid).as_ref() {
					Some(r) => Some(&r.0),
					None => None
				}				
			},
			Container::Val(v) => panic!("Expected List, got Val({:?})", v)
		}
	}

	pub fn get_val(&self, tid:usize) -> Option<&T> {
		match self {
			Container::Val(v) => unsafe {
				match v.read(tid).as_ref() {
					Some(r) => Some(&r.0),
					None => None
				}				
			},
			Container::List(l) => panic!("Expected Val, got List({:?})", l)
		}
	}

	pub fn count(&self) -> usize {
		match self {
			Container::List(l) => l.len(),
			_ => 1
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn container_new_list_works() {
    	set_epoch();
        let cont = Container::<TestType>::new_list(3);
        assert_eq!(cont.count(), 3);
        match cont {
        	Container::List(l) => {
		        assert!(l[0].read(0) == ptr::null_mut());
		        assert!(l[1].read(0) == ptr::null_mut());
		        assert!(l[2].read(0) == ptr::null_mut());
        	},
        	Container::Val(v) => panic!("Expected List, but got Val({:?})", v)
        }
    }

    #[test]
    fn container_set_list_works() {
    	set_epoch();
    	let cont = Container::<TestType>::new_list(2);
    	cont.set_list(0, TestType(10), 0);
    	cont.set_list(1, TestType(5), 0);
		match cont {
			Container::List(l) => unsafe {
		    	match l[0].read(0).as_ref() {
		    		Some(r) => assert_eq!(r.0.get_val(0).unwrap().0, 10),
		    		None => panic!("Expected {:?} at position {:?}, got nullptr", TestType(10), 0)
		    	}

		    	match l[1].read(0).as_ref() {
		    		Some(r) => assert_eq!(r.0.get_val(0).unwrap().0, 5),
		    		None => panic!("Expected {:?} at position {:?}, got nullptr", TestType(5), 1)
		    	}
			},
			Container::Val(v) => panic!("Expected List but got Val({:?})", v)
		}
    }

    #[test]
    fn container_set_val_works() {
    	set_epoch();
    	let cont = Container::<TestType>::new_list(2);
    	cont.set_list(0, TestType(6), 0);
    	match cont.get_list(0, 0) {
    		Some(r) => r.set_val(TestType(3), 0),
    		None => panic!("Expected val, but got nullptr")
    	}

    	match cont.get_list(0, 0) {
    		Some(r) => assert_eq!(r.get_val(0).unwrap().0, 3),
    		None => panic!("Expected val, but got nullptr")
    	}

    }
    
    #[test]
    fn container_get_list_works() {
    	set_epoch();
    	let cont = Container::<TestType>::new_list(2);
    	let value = 6;
    	cont.set_list(0, TestType(value), 0);
    	match cont.get_list(0, 0) {
    		Some(r) => assert_eq!(r.get_val(0).unwrap().0, value),
    		None => panic!("Expeted {:?}, got nullptr", TestType(value))
    	}
    }
}
