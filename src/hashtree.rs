use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::{ptr, thread};
use std::fmt::Debug;
use std::ops::Deref;
use crate::tlocal;
use crate::traits::NewType;
use crate::logging::*;

static DEFAULT_HASH_BASE:u64 = 0x5331;

#[derive(Debug)]
pub struct HashScheme( /*Base */u64);

impl HashScheme {
	fn hash(&self, data:&[u8], align:usize) -> u64 {
		match align {
			1 => {
				let mut base = self.0;
				for b in data.iter() {
					base = ((base << (*b & 0x2a)) | (base >> (*b & 0x2a))) ^ (*b as u64);
				}
				base
			},
			8 => {
				let mut base = self.0;

				let ptr = data.as_ptr() as *const u64;
				for i in 0..(data.len() / 8) {
					base = ((base << 29) | (base >> 29)) ^ (unsafe { *ptr.offset(i as isize) });
				}
				base
			},
			_ => panic!("Unsupported alignment for HashScheme: {:?}", align)
		}
	}

	fn evolve(&self) -> HashScheme {
		let tick = tlocal::time();
		HashScheme(self.0 ^ tick)
	}

	pub fn default() -> HashScheme {
		HashScheme(DEFAULT_HASH_BASE)
	}
}

#[derive(Debug)]
pub enum HashTree<T> {
	Table(HashScheme, Vec<AtomicPtr<HashTree<T>>>),
	Item(Box<[u8]>,  T, AtomicPtr<HashTree<T>>)
}

impl<T> Drop for HashTree<T> {
    fn drop(&mut self) {
    	match self {
    		HashTree::Item(_, _, ptr) => {
    			let next_ptr = ptr.load(Ordering::SeqCst);
    			if nonull!(next_ptr) {
    				free!(next_ptr);
    			}
    		},
    		HashTree::Table(_, ptr_list) => {
    			for i in 0..ptr_list.len() {
    				let next_ptr = ptr_list[i].load(Ordering::SeqCst);
    				if nonull!(next_ptr) {
    					free!(next_ptr);
    				}
    			}
    		}
    	}
    }
}

fn compare_aligned(lfs:&[u8], rfs:&[u8], align:usize) -> bool {
	match align {
		1 => lfs == rfs,
		8 => {
			if lfs.len() != rfs.len() {
				return false;
			} else {
				let lptr = lfs.as_ptr();
				let rptr = rfs.as_ptr();
				for i in 0..lfs.len() {
					unsafe {
						if *lptr.offset(i as isize) != *rptr.offset(i as isize) {
							return false;
						}
					}
				}
				return true;
			}
		}
		_ => panic!("Unsupported alignment for compare_aligned: {:?}", align)
	}
}

impl<T: Debug + NewType> HashTree<T> {
	pub fn new_table(hasher:HashScheme, slot_count:usize) -> HashTree<T> {
		let mut slots = vec![];
		slots.reserve(slot_count);
		for _ in 0..slot_count {
			slots.push(AtomicPtr::new(ptr::null_mut()));
		}
		HashTree::Table(hasher, slots)
	}

	pub fn value(&self) -> &T {
		match self {
			HashTree::Item(_, v, _) => return &v,
			HashTree::Table(_, _) => panic!("Atttempted to call value() on {:?}", self)
		}
	}

	pub fn new_item(key:&[u8]) -> *mut HashTree<T> {
		Box::into_raw(Box::new(HashTree::Item(key.into(), T::new(), AtomicPtr::new(ptr::null_mut()))))
	}

	pub fn find_string(&self, key:&str) -> Option<&T> {
		self.find_bytes(key.as_bytes(), 1)
	}

	pub fn find_bytes(&self, key:&[u8], align:usize) -> Option<&T> {
		match self {
			HashTree::Table(hasher, slots) => {
				let hashed_idx = hasher.hash(key, align) % (slots.len() as u64);
				let find_slot = &slots[hashed_idx as usize];
				let slot_ptr = find_slot.load(Ordering::SeqCst);
				if slot_ptr == ptr::null_mut() {
					return None;
				}
				let slot_ref = unsafe { slot_ptr.as_ref().unwrap() };
				match slot_ref {
					HashTree::Item(k, v, p) => {
						if compare_aligned(k.deref(), key, align) {
							return Some(v);
						}
						unsafe {
							match p.load(Ordering::SeqCst).as_ref() {
								Some(coll_ref) => {
									return coll_ref.find_bytes(key, align);
								},
								None => { return None; }
							}
						}
					},
					HashTree::Table(_, _) => panic!("Expected to find Item, got Table: {:?}", slot_ref)
				}		
			},
			HashTree::Item(_, _, _) => panic!("Expected Table, got Item: {:?}", self)
		}
	}

	pub fn insert_string(&self, key:&str) -> &T {
		self.insert_bytes(key.as_bytes(), 1)
	}

	pub fn insert_bytes(&self, key:&[u8], align:usize) -> &T {
		match self {
			HashTree::Table(hasher, slots) => {
				let hashed_idx = hasher.hash(key, align) % (slots.len() as u64);
				let insert_slot = &slots[hashed_idx as usize];
				let slot_ptr = insert_slot.load(Ordering::SeqCst);
				if slot_ptr != ptr::null_mut() {
					unsafe {
						let slot_ref = slot_ptr.as_ref().unwrap();
						match slot_ref {
							HashTree::Item(k, v, p) => {
								if compare_aligned(k.deref(), key, align) {
									return v;
								} else {
									// collission
									match p.load(Ordering::SeqCst).as_ref() {
										Some(coll_ref) => return coll_ref.insert_bytes(key, align),
										None => {
											let coll_table = Box::into_raw(
												                Box::new(HashTree::new_table(hasher.evolve(), slots.len())
												                	)
												                );
											match p.compare_exchange(ptr::null_mut(), coll_table, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) => return coll_table.as_ref().unwrap().insert_bytes(key, align),
												Err(p_seen) => {
												    drop(Box::from_raw(coll_table)); 
													return p_seen.as_ref().unwrap().insert_bytes(key, align); 
												}
											}
										}
									}
								}
							},
							HashTree::Table(_, _) => panic!("Expected item in slot, found table: {:?}", slot_ref)
						}
					}
				}
				// is nullptr, new slot
				let item_slot = HashTree::new_item(key);
				match insert_slot.compare_exchange(ptr::null_mut(), item_slot, Ordering::SeqCst, Ordering::SeqCst) {
					Ok(_) => unsafe {
						return item_slot.as_ref().unwrap().value();
					},
					Err(p_seen) => unsafe {
						drop(Box::from_raw(item_slot));
						let coll_ref = p_seen.as_ref().unwrap();
						match coll_ref {
							HashTree::Item(k, v, p) => {
								if compare_aligned(k.deref(), key, align) {
									// correct slot
									return &v;
								} else {
									match p.load(Ordering::SeqCst).as_ref() {
										Some(coll_ref) => return coll_ref.insert_bytes(key, align),
										None => {
											let coll_table = Box::into_raw(
												                Box::new(HashTree::new_table(hasher.evolve(), slots.len())
												                	)
												                );
											match p.compare_exchange(ptr::null_mut(), coll_table, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) => return coll_table.as_ref().unwrap().insert_bytes(key, align),
												Err(p_seen) => {
												    drop(Box::from_raw(coll_table)); 
													return p_seen.as_ref().unwrap().insert_bytes(key, align); 
												}
											}
										}
									}
								}
							},
							HashTree::Table(_, _) => panic!("Expected Item slot, but found: {:?}", coll_ref)
						}
					}
				}
			},
			HashTree::Item(_, _, _) => panic!("Attempted to call insert on item : {:?}", self)
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[derive(Debug)]
    struct TestType(AtomicU32);

    impl TestType {
    	fn set(&self, val:u32) {
    		self.0.store(val, Ordering::SeqCst);
    	} 

    	fn get(&self) -> u32 {
    		self.0.load(Ordering::SeqCst)
    	}
    }

    impl NewType for TestType {
    	fn new() -> Self {
    		TestType(AtomicU32::new(0))
    	}
    }

    #[test]
    fn evolve_hash_works() {
    	tlocal::set_epoch();
    	let hs = HashScheme::default();
    	let s = String::from("Hello!");
    	let hash1 = hs.hash(s.as_bytes(), 1);
    	let hs2 = hs.evolve();
    	let hash2 = hs2.hash(s.as_bytes(), 1);
    	assert!(hash1 != hash2);
    }

    #[test]
    fn evolve_hash_aligned_works() {
    	tlocal::set_epoch();
    	let hs = HashScheme::default();
    	let data:[u64;4] = [554, 334, 886, 9220];
    	let aligned = unsafe { data.align_to::<u8>() };
    	assert_eq!(aligned.0.len(), 0);
    	assert_eq!(aligned.2.len(), 0);
    	let hash1 = hs.hash(aligned.1, 8);
    	let hs2 = hs.evolve();
    	let hash2 = hs2.hash(aligned.1, 8);
    	assert!(hash1 != hash2);
    }

    #[test]
    fn insert_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let key = "Hello!";
    	let v = tree.insert_string(key);
    	v.set(6);
    	let v2 = tree.insert_string(key);
    	assert_eq!(v.get(), v2.get());
    	assert_eq!(v2.get(), 6);
    }

    #[test]
    fn insert_aligned_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let data:[u64;4] = [556, 332, 776, 4433];
    	let aligned = unsafe { data.align_to::<u8>() };
    	assert_eq!(aligned.0.len(), 0);
    	assert_eq!(aligned.2.len(), 0);
    	let key = aligned.1;
    	let v = tree.insert_bytes(key, 8);
    	v.set(6);
    	let v2 = tree.insert_bytes(key, 8);
    	assert_eq!(v.get(), v2.get());
    	assert_eq!(v2.get(), 6);
    }

    #[test]
    fn find_basic_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let key = "Hello!";
    	let v = tree.insert_string(key);
    	v.set(5);
    	let found = tree.find_string(key).unwrap();
    	assert_eq!(v.get(), found.get());
    	assert_eq!(found.get(), 5);
    }

    #[test]
    fn find_aligned_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let data:[u64;4] = [52376, 33122, 76376, 4433];
    	let aligned = unsafe { data.align_to::<u8>() };
    	assert_eq!(aligned.0.len(), 0);
    	assert_eq!(aligned.2.len(), 0);
    	let key = aligned.1;
    	let v = tree.insert_bytes(key, 8);
    	v.set(5);
    	let found = tree.find_bytes(key, 8).unwrap();
    	assert_eq!(v.get(), found.get());
    	assert_eq!(found.get(), 5);
    }

    #[test]
    fn mt_find_works() {
    	tlocal::set_epoch();
    	let mut tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let t1 = thcall!(5, 10, tree.insert_string("Hapy"));
    	let t2 = thcall!(5, 10, tree.insert_string("Sad"));
    	tree.insert_string("Happy");
    	match tree.find_string("Happy") {
    		None => panic!("Didn't find string 'Happy'"),
    		_ => ()
    	}
    	t1.join().unwrap();
    	t2.join().unwrap();
    }

    #[test]
    fn find_multi_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let key1 = "Hello!";
    	let key2 = "Hell3!";
    	let key3 = "Hell4!";
    	let v1 = tree.insert_string(key1);
    	let v2 = tree.insert_string(key2);
    	let v3 = tree.insert_string(key3);
    	v1.set(1);
    	v2.set(2);
    	v3.set(3);
    	assert_eq!(v1.get(), tree.find_string(key1).unwrap().get());
    	assert_eq!(v2.get(), tree.find_string(key2).unwrap().get());
    	assert_eq!(v3.get(), tree.find_string(key3).unwrap().get());
    }

    #[test]
    fn find_aligned_multi_works() {
    	tlocal::set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 10);
    	let data1:[u64;4] = [52373, 33129, 76376, 44773];
    	let aligned1 = unsafe { data1.align_to::<u8>() };
    	assert_eq!(aligned1.0.len(), 0);
    	assert_eq!(aligned1.2.len(), 0);
    	let key1 = aligned1.1;

    	let data2:[u64;4] = [523006, 331299, 7376, 433];
    	let aligned2 = unsafe { data2.align_to::<u8>() };
    	assert_eq!(aligned2.0.len(), 0);
    	assert_eq!(aligned2.2.len(), 0);
    	let key2 = aligned2.1;

    	let data3:[u64;4] = [526, 33122, 9976, 490033];
    	let aligned3 = unsafe { data3.align_to::<u8>() };
    	assert_eq!(aligned3.0.len(), 0);
    	assert_eq!(aligned3.2.len(), 0);
    	let key3 = aligned3.1;

    	let v1 = tree.insert_bytes(key1, 8);
    	let v2 = tree.insert_bytes(key2, 8);
    	let v3 = tree.insert_bytes(key3, 8);
    	v1.set(1);
    	v2.set(2);
    	v3.set(3);
    	assert_eq!(v1.get(), tree.find_bytes(key1, 8).unwrap().get());
    	assert_eq!(v2.get(), tree.find_bytes(key2, 8).unwrap().get());
    	assert_eq!(v3.get(), tree.find_bytes(key3, 8).unwrap().get());
    }
}