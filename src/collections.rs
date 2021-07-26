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
	fn new_item(key:&String) -> *mut BitTrie<T> {
		Box::into_raw(Box::new(BitTrie::Item(key.to_string(), 
			                                     shared::Shared::new(), 
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

	fn find(trie:*mut BitTrie<T>, key:&String) -> Option<&'static shared::Shared<Container<T>>> {
		unsafe {
			match trie.as_ref() {
				Some(r) => match r {
					BitTrie::Entry(hasher, childs) => {
						let mut cur_ptr = ptr::null_mut();
						let hash_seq = hasher.hash(key.as_bytes());
						let slot1 = &childs[(hash_seq & 1) as usize];
						let slot1_ptr = slot1.load(Ordering::SeqCst);
						if slot1_ptr == ptr::null_mut() {
							return None;
						}
						cur_ptr = slot1_ptr;
						for i in 1..64 {
							match cur_ptr.as_ref() {
								Some(rp) => match rp {
									BitTrie::Connect(cchilds) => {
										let slotc = &cchilds[((hash_seq >> i) & 1) as usize];
										let slotc_ptr = slotc.load(Ordering::SeqCst);
										if slotc_ptr == ptr::null_mut() {
											return None;
										}
										cur_ptr = slotc_ptr;
									},
									BitTrie::Item(_, _, _) => panic!("Expected connect node, but found Item: {:?}", rp),
									BitTrie::Entry(_, _) => panic!("Expected connect node, but found entry {:?}", rp)
								},
								None => { return None; }
							}
						}
						// ok we are at the item node now
						match cur_ptr.as_ref() {
							Some(ritem) => match ritem {
								BitTrie::Item(k, v, p) => {
									if k == key {
										return Some(&v);
									} else {
										// check for collision, proceed
										let next_gen_ptr = p.load(Ordering::SeqCst);
										if next_gen_ptr == ptr::null_mut() {
											// not yet present
											return None;
										} else {
											return BitTrie::find(next_gen_ptr, key);
										}
									}
								},
								BitTrie::Connect(_) => panic!("Expected item node but found connect: {:?}", ritem),
								BitTrie::Entry(_, _) => panic!("Expected item node but found entry: {:?}", ritem)
							},
							None => { return None; }
						}

					},
					BitTrie::Connect(_) => panic!("Attempted to call find on Connect node: {:?}", r),
					BitTrie::Item(_, _, _) => panic!("Attempted to call find on Item node: {:?}", r)
				},
				None => panic!("Expected trie for find but got nullptr")
			}
		}
	}
	
    fn insert(trie: *mut BitTrie<T>, key:&String, val:Container<T>, tid:usize, update:bool) {
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
										BitTrie::Connect(cchilds) => {
											let slotc = &cchilds[((hash_seq >> i) & 1) as usize];
											let slotc_ptr = slotc.load(Ordering::SeqCst);
											if slotc_ptr != ptr::null_mut() {
												cur_ptr = slotc_ptr;
											} else {
												let to_put = BitTrie::new_connect();
												match slotc.compare_exchange(ptr::null_mut(), to_put, Ordering::SeqCst, Ordering::SeqCst) {
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
									BitTrie::Connect(cchilds) => {
										let slotl = &cchilds[((hash_seq >> 63) & 1) as usize];
										let slotl_ptr = slotl.load(Ordering::SeqCst);
										if slotl_ptr != ptr::null_mut() {
											// item node is present , collision ?
											cur_ptr = slotl_ptr;
										} else {
											let to_insert_item = BitTrie::new_item(&key);
											match slotl.compare_exchange(ptr::null_mut(), to_insert_item, Ordering::SeqCst, Ordering::SeqCst) {
												Ok(_) =>  {
													match to_insert_item.as_ref().unwrap() {
														BitTrie::Item(_, sh, _) => sh.write(shared::TimePtr::make(val), tid),
														_ => panic!("Expected just created Item, got other variant")
													}
													return;
												},
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
										if k == key {
											if update {
												v.write(shared::TimePtr::make(val), tid);
											}
											return;
										} else {
											let next_gen_ptr = p.load(Ordering::SeqCst);
											if next_gen_ptr != ptr::null_mut() {
												BitTrie::insert(next_gen_ptr, key, val, tid, update);
											} else {
												let next_gen_node = BitTrie::new_gen_entry(hasher);
												match p.compare_exchange(ptr::null_mut(), 
													               next_gen_node, 
													               Ordering::SeqCst, 
													               Ordering::SeqCst) {
													Ok(_) => { BitTrie::insert(next_gen_node, key, val, tid, update); },
													Err(p) => { BitTrie::insert(p, key, val, tid, update); }
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

#[derive(Debug)]
enum HashTree<T> {
	Table(HashScheme, Vec<AtomicPtr<HashTree<T>>>),
	Item(String,  shared::Shared<Container<T>>, AtomicPtr<HashTree<T>>)
}

impl<T: Debug> HashTree<T> {
	fn new_table(hasher:HashScheme, slot_count:usize) -> HashTree<T> {
		let mut slots = vec![];
		slots.reserve(slot_count);
		for _ in 0..slot_count {
			slots.push(AtomicPtr::new(ptr::null_mut()));
		}
		HashTree::Table(hasher, slots)
	}

	fn value(&self) -> &shared::Shared<Container<T>> {
		match self {
			HashTree::Item(k, v, p) => return &v,
			HashTree::Table(_, _) => panic!("Atttempted to vall value() on {:?}", self)
		}
	}

	fn new_item(key:&String) -> *mut HashTree<T> {
		Box::into_raw(Box::new(HashTree::Item(key.clone(), shared::Shared::new(), AtomicPtr::new(ptr::null_mut()))))
	}

	fn insert(&self, key:&String) -> &shared::Shared<Container<T>> {
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

    #[test]
    fn bittrie_insert_find_works() {
    	shared::set_epoch();
    	let base = BitTrie::new_entry();
    	let key = String::from("Hello!");
    	let value = Container::new_list(1);
    	let inner_value = Container::Val(10);
    	value.set_list(0, inner_value, 0);
    	BitTrie::insert(base, &key, value, 0, false);
    	unsafe {
	    	match BitTrie::find(base, &key) {
	    		Some(rshared) => match rshared.read(0).as_ref() {
	    			Some(readr) => {
	    				assert_eq!(*readr.0.get_list(0, 0).unwrap().value(), 10);
	    			},
	    			None => panic!("Expected readable ptr for key: {:?}, got nullptr", &key)
	    		},
	    		None => panic!("Tried to insert {:?}, was not found", &key)
	    	}
    	}
    }
}