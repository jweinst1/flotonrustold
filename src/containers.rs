use std::fmt::Debug;
use crate::shared::*;
use crate::tlocal;
use crate::hashtree::{HashTree, HashScheme};
use crate::logging::*;
use crate::traits::*;

#[derive(Debug)]
pub enum ContainerErr {
    KeyNotFound
}

#[derive(Debug)]
pub enum Container<T> {
	Val(T),
	Map(HashTree<Shared<Container<T>>>)
}

fn hash_tree_cont_output_binary<T: InPutOutPut>(tree:&HashTree<Shared<Container<T>>>, 
                                                output: &mut Vec<u8>) {
    match tree {
        HashTree::Table(_, tble) => for i in 0..tble.len() {
            let tptr = tble[i].load(Ordering::SeqCst);
            if nonull!(tptr) {
                unsafe {
                    match tptr.as_ref().unwrap() {
                        HashTree::Item(ikey, ival, iother) => {
                            let current_val = ival.read();
                            if nonull!(current_val) {
                                // 1 byte for size, for now
                                output.push(ikey.len() as u8);
                                for j in 0..ikey.len() {
                                    output.push(ikey[j]);
                                }
                                current_val.as_ref().unwrap().0.output_binary(output);
                            }
                            // also add collided key-value pairs
                            let other_ptr = iother.load(Ordering::SeqCst);
                            if nonull!(other_ptr) {
                                hash_tree_cont_output_binary(other_ptr.as_ref().unwrap(), output);
                            }
                        },
                        HashTree::Table(gen, _) => {
                            log_fatal!(Output, "Unexpected table in place of item during format, gen: {:?}", gen);
                            panic!("Cannot format container with invalid item");
                        }
                    }
                }
            }
        },
        HashTree::Item(key, _, _) => {
            log_fatal!(Output, "Unexpected item in place of table during format, key: {:?}", key);
            panic!("Cannot format container with invalid map");
        }
    }
} 

impl<T: InPutOutPut> InPutOutPut for Container<T> {
    fn output_binary(&self, output: &mut Vec<u8>) {
        match self {
            Container::Val(v) => v.output_binary(output),
            Container::Map(m) => hash_tree_cont_output_binary(m, output)
        }
    }
    fn input_binary(input:&[u8], place:&mut usize) -> Self {
        Container::new_map(30)
    }
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
