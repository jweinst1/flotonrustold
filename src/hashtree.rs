use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, Ordering};
use std::ptr;
use std::fmt::Debug;
use std::convert::TryFrom;
use crate::epoch::{set_epoch, check_time};

static DEFAULT_HASH_BASE:u64 = 0x5331;

#[derive(Debug)]
pub struct HashScheme( /*Base */u64);

impl HashScheme {
	fn hash(&self, data:&[u8]) -> u64 {
		let mut base = self.0;
		for b in data.iter() {
			base = ((base << (*b & 0x2a)) | (base >> (*b & 0x2a))) ^ (*b as u64);
		}
		base
	}

	fn evolve(&self) -> HashScheme {
		let tick = check_time();
		HashScheme(self.0 ^ tick)
	}

	fn default() -> HashScheme {
		HashScheme(DEFAULT_HASH_BASE)
	}
}

pub trait HashTreeType {
	fn new() -> Self;
}

#[derive(Debug)]
enum HashTree<T> {
	Table(HashScheme, Vec<AtomicPtr<HashTree<T>>>),
	Item(String,  T, AtomicPtr<HashTree<T>>)
}

impl<T: Debug + HashTreeType> HashTree<T> {
	fn new_table(hasher:HashScheme, slot_count:usize) -> HashTree<T> {
		let mut slots = vec![];
		slots.reserve(slot_count);
		for _ in 0..slot_count {
			slots.push(AtomicPtr::new(ptr::null_mut()));
		}
		HashTree::Table(hasher, slots)
	}

	fn value(&self) -> &T {
		match self {
			HashTree::Item(_, v, _) => return &v,
			HashTree::Table(_, _) => panic!("Atttempted to vall value() on {:?}", self)
		}
	}

	fn new_item(key:&String) -> *mut HashTree<T> {
		Box::into_raw(Box::new(HashTree::Item(key.clone(), T::new(), AtomicPtr::new(ptr::null_mut()))))
	}

	fn find(&self, key:&String) -> Option<&T> {
		match self {
			HashTree::Table(hasher, slots) => {
				let hashed_idx = hasher.hash(key.as_bytes()) % (slots.len() as u64);
				let find_slot = &slots[hashed_idx as usize];
				let slot_ptr = find_slot.load(Ordering::SeqCst);
				if slot_ptr == ptr::null_mut() {
					return None;
				}
				let slot_ref = unsafe { slot_ptr.as_ref().unwrap() };
				match slot_ref {
					HashTree::Item(k, v, p) => {
						if k == key {
							return Some(v);
						}
						unsafe {
							match p.load(Ordering::SeqCst).as_ref() {
								Some(coll_ref) => {
									return coll_ref.find(key);
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

	fn insert(&self, key:&String) -> &T {
		match self {
			HashTree::Table(hasher, slots) => {
				let hashed_idx = hasher.hash(key.as_bytes()) % (slots.len() as u64);
				let insert_slot = &slots[hashed_idx as usize];
				let slot_ptr = insert_slot.load(Ordering::SeqCst);
				if slot_ptr != ptr::null_mut() {
					unsafe {
						let slot_ref = slot_ptr.as_ref().unwrap();
						match slot_ref {
							HashTree::Item(k, v, p) => {
								if k == key {
									return v;
								} else {
									// collission
									match p.load(Ordering::SeqCst).as_ref() {
										Some(coll_ref) => return coll_ref.insert(key),
										None => {
											let coll_table = Box::into_raw(
												                Box::new(HashTree::new_table(hasher.evolve(), slots.len())
												                	)
												                );
											match p.compare_exchange(ptr::null_mut(), coll_table, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) => return coll_table.as_ref().unwrap().insert(key),
												Err(p_seen) => {
												    drop(Box::from_raw(coll_table)); 
													return p_seen.as_ref().unwrap().insert(key); 
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
								if k == key {
									// correct slot
									return &v;
								} else {
									match p.load(Ordering::SeqCst).as_ref() {
										Some(coll_ref) => return coll_ref.insert(key),
										None => {
											let coll_table = Box::into_raw(
												                Box::new(HashTree::new_table(hasher.evolve(), slots.len())
												                	)
												                );
											match p.compare_exchange(ptr::null_mut(), coll_table, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) => return coll_table.as_ref().unwrap().insert(key),
												Err(p_seen) => {
												    drop(Box::from_raw(coll_table)); 
													return p_seen.as_ref().unwrap().insert(key); 
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
    #[derive(Debug)]
    struct TestType(AtomicU32);

    impl TestType {
    	fn set(&self, val:u32) {
    		self.0.store(val, Ordering::SeqCst);
    	} 

    	fn get(&self) -> u32 {
    		self.0.load(Ordering::SeqCst)
    	}

    	fn new(val:u32) -> TestType {
    		TestType(AtomicU32::new(val))
    	}
    }

    impl HashTreeType for TestType {
    	fn new() -> Self {
    		TestType(AtomicU32::new(0))
    	}
    }

    #[test]
    fn evolve_hash_works() {
    	set_epoch();
    	let hs = HashScheme::default();
    	let s = String::from("Hello!");
    	let hash1 = hs.hash(s.as_bytes());
    	let hs2 = hs.evolve();
    	let hash2 = hs2.hash(s.as_bytes());
    	assert!(hash1 != hash2);
    }

    #[test]
    fn hashtree_insert_works() {
    	set_epoch();
    	let tree = HashTree::<TestType>::new_table(HashScheme::default(), 50);
    	let key = String::from("Hello!");
    	let v = tree.insert(&key);
    	v.set(6);
    	let v2 = tree.insert(&key);
    	assert_eq!(v.get(), v2.get());
    	assert_eq!(v2.get(), 6);
    }
}