use std::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;

pub struct KVPair<T>(String, SharedPtr<T>);

impl<T> KVPair<T> {
	fn new(key:&str, value:T) -> KVPair<T> {
		KVPair(String::from(key), SharedPtr::new(Some(value)))
	}

	fn hash_32(&self) -> u32 {
		let mut digest:u32 = 5381;
		for c in self.0.as_bytes() {
			digest = ((digest << 5) + digest) + *c as u32;
		}
		return digest;
	}
}
/*
pub struct KVSlice<T> {
	items:Vec<SharedPtr<KVPair<T>>>,
	occupied:AtomicU32,
	next:AtomicPtr<KVSlice<T>>
}

impl<T> KVSlice<T> {
	pub fn new(size:usize) -> KVSlice<T> {
		let mut slots = vec![];
		for _ in 0..size {
			slots.push(SharedPtr::new(None));
		}
		KVSlice{items:slots, occupied:AtomicU32::new(0), next:AtomicPtr::new(ptr::null_mut())}
	}

	pub fn load_factor(&self) -> f32 {
		self.occupied.load(Ordering::SeqCst) as f32 / self.items.len() as f32
	}
}


pub struct KVMap<T> {
	slice_size:usize,
	slices:KVSlice<T>,
	slice_count:AtomicUsize
}

impl<T> KVMap<T> {
	pub fn new(size:usize) -> KVMap<T> {
		KVMap{slice_size:size, slices:KVSlice::new(size), slice_count::AtomicUsize::new(1)}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
    }
}*/
