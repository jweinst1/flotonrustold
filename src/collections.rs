use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::ptr;
use std::fmt::Debug;
use std::time::{Instant};
use std::convert::TryFrom;
use std::mem::{self, MaybeUninit};
use crate::shared;
use crate::containers::Container;

static DEFAULT_HASH_BASE:u64 = 0x5331;

#[derive(Debug)]
struct HashScheme( /*Base */u64);

impl HashScheme {
	fn hash(&self, data:&[u8]) -> u64 {
		let mut base = self.0;
		for b in data.iter() {
			base = ((base << (*b & 0x2a)) | (base >> (*b & 0x2a))) ^ (*b as u64);
		}
		base
	}

	fn evolve(&self) -> HashScheme {
		let tick = shared::check_time();
		HashScheme(self.0 ^ tick)
	}
}

#[derive(Debug)]
enum BitTrie<T> {
	Connect([AtomicPtr<BitTrie<T>>; 2]),
	Entry(HashScheme, [AtomicPtr<BitTrie<T>>; 2]),
	Item(String, shared::Shared<Container<T>>, /*Entry of next layer*/ AtomicPtr<BitTrie<T>>)
}

impl<T: Debug> BitTrie<T> {
	fn new_item(key:&String, val:Container<T>) -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Item(key.to_string(), 
			                                     shared::Shared::new_val(val), 
			                                     AtomicPtr::new(ptr::null_mut()))))
	}

	fn new_connect() -> *mut BitTrie<T> {
		Box::into_raw(Box::new(
			             BitTrie::Connect([ 
			             	          AtomicPtr::new(ptr::null_mut()), 
			             	          AtomicPtr::new(ptr::null_mut())
			             	                  ])
			             ))
	}

	fn new_entry() -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Entry(
			                 HashScheme(DEFAULT_HASH_BASE), [ 
			             	          AtomicPtr::new(ptr::null_mut()), 
			             	          AtomicPtr::new(ptr::null_mut())
			             	                  ]
			             	          )
		))
	}

	fn new_gen_entry(hasher:&HashScheme) -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Entry(
			                 hasher.evolve(), [ 
			             	          AtomicPtr::new(ptr::null_mut()), 
			             	          AtomicPtr::new(ptr::null_mut())
			             	                  ]
			             	          )
		))
	}

	fn insert(trie: *mut BitTrie<T>, key:String, val:Container<T>) {
		unsafe {
		match trie.as_ref() {
			Some(r) => {
				match r {
					BitTrie::Entry(hasher, childs) => {
						let mut cur_ptr = ptr::null_mut();
						let hash_seq = hasher.hash(key.as_bytes());
						let slot1 = &childs[(hash_seq & 1) as usize];
						let slot1_ptr = slot1.load(Ordering::SeqCst);
						if slot1_ptr != ptr::null_mut() {
							cur_ptr = slot1_ptr;
						} else {
							let to_put = BitTrie::new_connect();
							match slot1.compare_exchange(ptr::null_mut(), to_put, Ordering::SeqCst, Ordering::SeqCst) {
								Ok(_) => cur_ptr = to_put,
								Err(p) => {
									drop(Box::from_raw(to_put));
									cur_ptr = p; 
								}
							} 
						}
						for i in 1..63 {
							match cur_ptr.as_ref() {
								Some(rcon) => {
									match rcon {
										BitTrie::Connect(childs) => {
											let slotc = &childs[((hash_seq >> i) & 1) as usize];
											let slotc_ptr = slotc.load(Ordering::SeqCst);
											if slotc_ptr != ptr::null_mut() {
												cur_ptr = slotc_ptr;
											} else {
												let to_put = BitTrie::new_connect();
												match slot1.compare_exchange(ptr::null_mut(), to_put, Ordering::SeqCst, Ordering::SeqCst) {
													Ok(_) => cur_ptr = to_put,
													Err(p) => {
														drop(Box::from_raw(to_put));
														cur_ptr = p; 
													}
												} 
											}
										},
										BitTrie::Entry(_, _) => panic!("Found entry node before {:?} chains", 64),
										BitTrie::Item(_, _, _) => panic!("Found item node before {:?} chains", 64)
									}
								},
								None => panic!("Expected valid pointer, got nullptr")
							}
						}
						// last one, need to connect to item node
						match cur_ptr.as_ref() {
							Some(rlast) => {
								match rlast {
									BitTrie::Connect(childs) => {
										let slotl = &childs[((hash_seq >> 63) & 1) as usize];
										let slotl_ptr = slotl.load(Ordering::SeqCst);
										if slotl_ptr != ptr::null_mut() {
											// item node is present , collision ?
											cur_ptr = slotl_ptr;
										} else {
											let to_insert_item = BitTrie::new_item(&key, val);
											match slotl.compare_exchange(ptr::null_mut(), to_insert_item, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) =>  {  return; },
												Err(p) => {
													drop(Box::from_raw(to_insert_item));
													// collision
													cur_ptr = p; 
												}
											} 
										}
									},
									BitTrie::Entry(_, _) => panic!("Found entry node before {:?} chains", 64),
									BitTrie::Item(_, _, _) => panic!("Found item node before {:?} chains", 64)
								}
							}
							None => panic!("Expected valid connection pointer, got nullptr")
						}
						// this means we have a collision, we need to check if key is equal
						match cur_ptr.as_ref() {
							Some(rcolls) => {
								match rcolls {
									BitTrie::Item(k, v, p) => {
										if k == &key {
											// not an update, time to return
											return;
										} else {
											let next_gen_ptr = p.load(Ordering::SeqCst);
											if next_gen_ptr != ptr::null_mut() {
												BitTrie::insert(next_gen_ptr, key, val);
											} else {
												let next_gen_node = BitTrie::new_gen_entry(hasher);
												match p.compare_exchange(ptr::null_mut(), 
													               next_gen_node, 
													               Ordering::SeqCst, 
													               Ordering::SeqCst) {
													Ok(_) => { BitTrie::insert(next_gen_node, key, val); },
													Err(p) => { BitTrie::insert(p, key, val); }
												}
											}
										}
									},
									BitTrie::Connect(_) => panic!("Expected Item node but found {:?}", rcolls),
									BitTrie::Entry(_, _) => panic!("Expected Item node but found {:?}", rcolls)
								}
							},
							None => panic!("Expected valid item pointer, got nullptr")
						}
					},
					BitTrie::Connect(_) => panic!("Attempted to call insert on Connect node: {:?}", r),
					BitTrie::Item(_, _, _) => panic!("Attempted to call insert on Item node: {:?}", r)
				}
			},
			None => panic!("Attempted to call insert on nullptr")
		}
	}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn evolve_hash_works() {
    	shared::set_epoch();
    	let hs = HashScheme(DEFAULT_HASH_BASE);
    	let s = String::from("Hello!");
    	let hash1 = hs.hash(s.as_bytes());
    	let hs2 = hs.evolve();
    	let hash2 = hs2.hash(s.as_bytes());
    	assert!(hash1 != hash2);
    }
}