use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;
use crate::bit_trie::BitTrie;
/*
#[derive(Debug)]
struct List<T> {
	items:BitTrie<SharedPtr<T>>,
	len:AtomicUsize
}*/

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
    }
}*/
