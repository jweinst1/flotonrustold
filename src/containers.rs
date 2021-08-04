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

    pub fn value(&self) -> &T {
        match self {
            Container::Val(v) => &v,
            Container::Map(m) => panic!("Called value() on map: {:?}", m)
        }
    }

	pub fn set_map(&self, key:&[u8], val:Container<T>, tid:usize) {
		match self {
			Container::Val(v) => panic!("Expected List, got Val({:?})", v),
			Container::Map(m) => m.insert_bytes(key).write(TimePtr::make(val), tid)
		}
	}

    pub fn get_map_shared(&self, key:&[u8]) -> Option<&Shared<Container<T>>> {
        match self {
            Container::Val(v) => panic!("Expected List, got Val({:?})", v),
            Container::Map(m) => m.find_bytes(key)
        }
    }

    pub fn get_map(&self, key:&[u8], tid:usize) -> Option<&Container<T>> {
        match self {
            Container::Val(v) => panic!("Expected List, got Val({:?})", v),
            Container::Map(m) => match m.find_bytes(key) {
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
    fn set_map_works() {
    	set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val, 0);
        match map {
            Container::Map(m) => match m.find_bytes(key) {
                Some(r) => unsafe { match r.read(0).as_ref() {
                    Some(rval) => assert_eq!(rval.0.value().0, 10),
                    None => panic!("Unexpected nullptr from shared loc {:?}", r)
                 } },
                None => panic!("Expected map {:?} to contain value for key {:?}", m, key)
            }
            Container::Val(v) => panic!("Unexpected Value({:?})", v)
        }
    }

    #[test]
    fn get_map_shared_works() {
        set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val, 0);
        match map.get_map_shared(key) { Some(rsh) => unsafe {  
            match rsh.read(0).as_ref() { 
                Some(rval) =>  assert_eq!(rval.0.value().0, 10), 
                None => panic!("Unexpected nullptr from shared loc {:?}", rsh) 
            }
            }, 
            None => panic!("key: {:?} not found in map {:?}", key, map)
        }
    }

    #[test]
    fn get_map_works() {
        set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val, 0);
        match map.get_map(key, 0) {
            Some(rv) => assert_eq!(rv.value().0, 10),
            None => panic!("key: {:?} not found in map {:?}", key, map)
        } 
    }
}
