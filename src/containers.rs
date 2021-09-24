use std::fmt::Debug;
use std::sync::atomic::Ordering;
use std::ptr;
use crate::shared::*;
use crate::tlocal;
use crate::hashtree::{HashTree, HashScheme};
use crate::logging::*;
use crate::traits::*;
use crate::errors::FlotonErr;
use crate::constants::{VBIN_CMAP_BEGIN, VBIN_CMAP_END, CMAPB_KEY};

#[derive(Debug)]
pub enum Container<T> {
	Val(T),
	Map(HashTree<Shared<Container<T>>>)
}

fn hash_tree_cont_output_binary<T: InPutOutPut + Debug>(tree:&HashTree<Shared<Container<T>>>, 
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
                                // annotates a key
                                output.push(CMAPB_KEY);
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

impl<T: InPutOutPut + Debug> InPutOutPut for Container<T> {
    fn output_binary(&self, output: &mut Vec<u8>) {
        match self {
            Container::Val(v) => v.output_binary(output),
            Container::Map(m) => {
                output.push(VBIN_CMAP_BEGIN);
                hash_tree_cont_output_binary(m, output);
                output.push(VBIN_CMAP_END);
            }
        }
    }

    fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr>  {
        if input[*place] == VBIN_CMAP_BEGIN {
            *place += 1;
            let nmap = Container::new_map(40); // todo make configurable
            while input[*place] != VBIN_CMAP_END {
                if input[*place] == CMAPB_KEY {
                    *place += 1;
                    let ksize = input[*place] as usize;
                    *place += 1;
                    let kslice = &input[*place..(*place + ksize)];
                    *place += ksize;
                    match Container::input_binary(input, place) {
                        Ok(val) => nmap.set_map(kslice, val),
                        Err(e) => return Err(e)
                    }
                } else {
                    log_error!(Input, "Invalid byte for container: {}", input[*place]);
                    return Err(FlotonErr::UnexpectedByte(input[*place]));
                }
            }
            *place += 1; // move past end
            return Ok(nmap);
        } else {
            return match T::input_binary(input, place) {
                Ok(r) => Ok(Container::Val(r)),
                Err(e) => Err(e)
            }
        }
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

    #[derive(Debug)]
    enum TestData {
        A,
        B
    }

    const TEST_DATA_A:u8 = 10;
    const TEST_DATA_B:u8 = 20;

    impl InPutOutPut for TestData {
        fn output_binary(&self, output: &mut Vec<u8>) {
            match self {
                TestData::A => output.push(TEST_DATA_A),
                TestData::B => output.push(TEST_DATA_B)
            }
        }
        fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr> {
            let byte = input[*place];
            *place += 1;
            match byte {
                TEST_DATA_A => Ok(TestData::A),
                TEST_DATA_B => Ok(TestData::B),
                _ => Err(FlotonErr::UnexpectedByte(byte))
            }
        }
    }

    #[test]
    fn tdata_container_output() {
        tlocal::set_epoch();
        let key1 = [11, 22, 33];
        let key2 = [11, 33, 44];
        let map = Container::new_map(10);
        map.set_map(&key1, Container::Val(TestData::A));
        map.set_map(&key2, Container::Val(TestData::A));
        let mut out_vec = vec![]; 
        map.output_binary(&mut out_vec);
        assert_eq!(out_vec.len(), 14);
        assert_eq!(out_vec[0], VBIN_CMAP_BEGIN);
        assert_eq!(out_vec[1], CMAPB_KEY);
        assert_eq!(out_vec[2], 3);
        assert_eq!(out_vec[6], TEST_DATA_A);
        assert_eq!(out_vec[7], CMAPB_KEY);
        assert_eq!(out_vec[8], 3);
        assert_eq!(out_vec[12], TEST_DATA_A);
        assert_eq!(out_vec[13], VBIN_CMAP_END);
    }

    #[test]
    fn tdata_overwrite_output() {
        tlocal::set_epoch();
        let key1 = [11, 22, 33];
        let key2 = [11, 22, 33];
        let map = Container::new_map(10);
        map.set_map(&key1, Container::Val(TestData::A));
        map.set_map(&key2, Container::Val(TestData::B));
        let mut out_vec = vec![]; 
        map.output_binary(&mut out_vec);
        assert_eq!(out_vec.len(), 8);
        assert_eq!(out_vec[0], VBIN_CMAP_BEGIN);
        assert_eq!(out_vec[1], CMAPB_KEY);
        assert_eq!(out_vec[2], 3);
        assert_eq!(out_vec[3], 11);
        assert_eq!(out_vec[4], 22);
        assert_eq!(out_vec[5], 33);
        assert_eq!(out_vec[6], TEST_DATA_B);
        assert_eq!(out_vec[7], VBIN_CMAP_END);
    }

    #[test]
    fn tdata_container_input() {
        tlocal::set_epoch();
        let key1 = [66, 77];
        let key2 = [128, 77];
        let input_bytes = [VBIN_CMAP_BEGIN, 
                           CMAPB_KEY, 2, key1[0], key1[1], TEST_DATA_B, 
                           CMAPB_KEY, 2, key2[0], key2[1], TEST_DATA_A, 
                           VBIN_CMAP_END];
        let mut i = 0;
        let parsed_map = Container::input_binary(&input_bytes, &mut i).expect("Cannot parse map from bytes");
        assert_eq!(i, 12);
        match parsed_map.get_map(&key1) {
            Some(contref) => match contref {
                Container::Val(v) => match v {
                    TestData::B => println!("{:?} passes", v),
                    TestData::A => panic!("Expected B, but got A")
                },
                Container::Map(m) => panic!("Expected parsed value. got map: {:?}", m)
            },
            None => panic!("Expected value in parsed map for key {:?}", key1)
        }

        match parsed_map.get_map(&key2) {
            Some(contref) => match contref {
                Container::Val(v) => match v {
                    TestData::A => println!("{:?} passes", v),
                    TestData::B => panic!("Expected A, but got B")
                },
                Container::Map(m) => panic!("Expected parsed value. got map: {:?}", m)
            },
            None => panic!("Expected value in parsed map for key {:?}", key1)
        }
    }

    #[test]
    fn tdata_overwrite_input() {
        tlocal::set_epoch();
        let key1 = [128, 77];
        let input_bytes = [VBIN_CMAP_BEGIN, 
                           CMAPB_KEY, 2, key1[0], key1[1], TEST_DATA_B, 
                           CMAPB_KEY, 2, key1[0], key1[1], TEST_DATA_A, 
                           VBIN_CMAP_END];
        let mut i = 0;
        let parsed_map = Container::input_binary(&input_bytes, &mut i).expect("Cannot parse map from bytes");
        assert_eq!(i, 12);
        match parsed_map.get_map(&key1) {
            Some(contref) => match contref {
                Container::Val(v) => match v {
                    TestData::A => println!("{:?} passes", v),
                    TestData::B => panic!("Expected A, but got B")
                },
                Container::Map(m) => panic!("Expected parsed value. got map: {:?}", m)
            },
            None => panic!("Expected value in parsed map for key {:?}", key1)
        }
    }

    #[test]
    fn nested_input() {
        tlocal::set_epoch();
        let key1 = [200, 53];
        let key2 = [100, 40];
        let input_bytes = [VBIN_CMAP_BEGIN,
                           CMAPB_KEY, 2, key1[0], key1[1], TEST_DATA_A,
                           CMAPB_KEY, 2, key2[0], key2[1], VBIN_CMAP_BEGIN,
                                                           CMAPB_KEY, 2, key1[0], key1[1], TEST_DATA_B,
                                                           VBIN_CMAP_END,
                           VBIN_CMAP_END];
        let mut i = 0;
        let parsed_map = Container::input_binary(&input_bytes, &mut i).expect("Cannot parse map from bytes");
        assert_eq!(i, 18);
        match parsed_map.get_map(&key1) {
            Some(contref) => match contref {
                Container::Val(v) => match v {
                    TestData::A => println!("{:?} passes", v),
                    TestData::B => panic!("Expected A, but got B")
                },
                Container::Map(m) => panic!("Expected parsed value. got map: {:?}", m)
            },
            None => panic!("Expected value in parsed map for key {:?}", key1)
        }

        match parsed_map.get_map(&key2) {
            Some(contref) => match contref {
                Container::Map(_) => match contref.get_map(&key1) {
                    Some(contv) => match contv {
                        Container::Val(v) => match v {
                            TestData::B => println!("{:?} passes", v),
                            TestData::A => panic!("Expected B but got A")
                        },
                        Container::Map(m) => panic!("Expected inner nest value, got map: {:?}", m)
                    },
                    None => panic!("Expected value for inner nested key {:?}", key1)
                },
                Container::Val(v) => panic!("Expected parsed map. got val: {:?}", v)
            },
            None => panic!("Expected value in parsed map for key {:?}", key1)
        }
    }

    #[test]
    fn nested_output() {
        tlocal::set_epoch();
        let key1 = [11, 22, 33];
        let map = Container::new_map(10);
        let nmap = Container::new_map(10);
        nmap.set_map(&key1, Container::Val(TestData::A));
        map.set_map(&key1, nmap);
        let mut out_vec = vec![]; 
        map.output_binary(&mut out_vec);
        assert_eq!(out_vec.len(), 15);
        assert_eq!(out_vec[0], VBIN_CMAP_BEGIN);
        assert_eq!(out_vec[1], CMAPB_KEY);
        assert_eq!(out_vec[2], 3);
        assert_eq!(out_vec[3], 11);
        assert_eq!(out_vec[4], 22);
        assert_eq!(out_vec[5], 33);
        assert_eq!(out_vec[6], VBIN_CMAP_BEGIN);
        assert_eq!(out_vec[7], CMAPB_KEY);
        assert_eq!(out_vec[8], 3);
        assert_eq!(out_vec[9], 11);
        assert_eq!(out_vec[10], 22);
        assert_eq!(out_vec[11], 33);
        assert_eq!(out_vec[12], TEST_DATA_A);
        assert_eq!(out_vec[13], VBIN_CMAP_END);
        assert_eq!(out_vec[14], VBIN_CMAP_END);
    }
}
