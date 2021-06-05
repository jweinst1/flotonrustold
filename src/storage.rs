use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;
use crate::bit_trie::BitTrie;
use crate::cache::Cache;

pub struct Storage<T> {
	items:BitTrie<SharedPtr<T>>,
	len:AtomicUsize,
	empty_slots:Cache<SharedPtr<T>>
}

impl<T> Storage<T> {
	pub fn new() -> Storage<T> {
		Storage{items:BitTrie::new(), len:AtomicUsize::new(0)}
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
    	
    }
}