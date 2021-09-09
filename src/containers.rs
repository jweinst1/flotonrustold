use std::fmt::Debug;
use crate::shared::*;
use crate::tlocal;
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

	pub fn set_map(&self, key:&[u8], val:Container<T>) {
		match self {
			Container::Val(v) => panic!("Expected Map, got Val({:?})", v),
			Container::Map(m) => m.insert_bytes(key).write(TimePtr::make(val))
		}
	}

    pub fn create_set_map(&self, key:&[u8], slots_size:usize) -> &Container<T> {
        match self {
            Container::Val(v) => panic!("Expected Map, got Val({:?})", v),
            Container::Map(m) => {
                let location = m.insert_bytes(key);
                // first, check if map already exists
                unsafe {
                    match location.read().as_ref() {
                        Some(loc_r) => match loc_r.0 {
                            Container::Map(_) => return &loc_r.0,
                            Container::Val(_) => () // can overwrite a val, proceed.
                        },
                        None => () // proceed to write
                    }
                }
                location.write(TimePtr::make(Container::new_map(slots_size)));
                // Do another read. This helps get the most up to date value.
                unsafe {
                    &location.read().as_ref().unwrap().0
                }
            }
        }
    }

    pub fn get_map_shared(&self, key:&[u8]) -> Option<&Shared<Container<T>>> {
        match self {
            Container::Val(v) => panic!("Expected Map, got Val({:?})", v),
            Container::Map(m) => m.find_bytes(key)
        }
    }

    pub fn get_map(&self, key:&[u8]) -> Option<&Container<T>> {
        match self {
            Container::Val(v) => panic!("Expected Map, got Val({:?})", v),
            Container::Map(m) => match m.find_bytes(key) {
                Some(refval) => unsafe { match refval.read().as_ref() {
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
    	tlocal::set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val);
        match map {
            Container::Map(m) => match m.find_bytes(key) {
                Some(r) => unsafe { match r.read().as_ref() {
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
        tlocal::set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val);
        match map.get_map_shared(key) { Some(rsh) => unsafe {  
            match rsh.read().as_ref() { 
                Some(rval) =>  assert_eq!(rval.0.value().0, 10), 
                None => panic!("Unexpected nullptr from shared loc {:?}", rsh) 
            }
            }, 
            None => panic!("key: {:?} not found in map {:?}", key, map)
        }
    }

    #[test]
    fn get_map_works() {
        tlocal::set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let val = Container::Val(TestType(10));
        map.set_map(key, val);
        match map.get_map(key) {
            Some(rv) => assert_eq!(rv.value().0, 10),
            None => panic!("key: {:?} not found in map {:?}", key, map)
        } 
    }

    #[test]
    fn create_set_map_works() {
        tlocal::set_epoch();
        let map = Container::new_map(20);
        let key = b"test";
        let key2 = b"test2";
        let val = Container::Val(TestType(10));
        let created = map.create_set_map(key, 30);
        match created {
            Container::Val(v) => panic!("Expected Map to be returned, got Val({:?})", v),
            Container::Map(_) => () // This is expected
        }
        created.set_map(key2, val);
        // test for overwrite
        match map.create_set_map(key, 30).get_map(key2) {
            Some(val_c) => assert_eq!(val_c.value().0, 10),
            None => panic!("Expected to find key {:?} nested in key {:?}", key2, key)
        }
    }
}
