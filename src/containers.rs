use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use crate::shared::*;
use crate::epoch::set_epoch;
use crate::traits::NewType;
use crate::hashtree::{HashTree, HashScheme};

#[derive(Debug)]
pub enum Container<T> {
	Val(T),
	Map(HashTree<Shared<Container<T>>>)
}

impl<T: Debug> Container<T> {
	pub fn new_map(size:usize) -> Container<T> {
		Container::Map(HashTree::new_table(HashScheme::default(), size))
	}

	pub fn set_map(&self, key:&String, val:Container<T>, tid:usize) {
		match self {
			Container::Val(v) => panic!("Expected List, got Val({:?})", v),
			Container::Map(m) => m.insert(key).write(TimePtr::make(val), tid)
		}
	}

    pub fn get_map_shared(&self, key:&String) -> Option<&Shared<Container<T>>> {
        match self {
            Container::Val(v) => panic!("Expected List, got Val({:?})", v),
            Container::Map(m) => m.find(key)
        }
    }

    pub fn get_map(&self, key:&String, tid:usize) -> Option<&Container<T>> {
        match self {
            Container::Val(v) => panic!("Expected List, got Val({:?})", v),
            Container::Map(m) => match m.find(key) {
                Some(refval) => unsafe { match refval.read(tid).as_ref() {
                    Some(r) => Some(&r.0),
                    None => None
                }},
                None => None
            }
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

    }
}
