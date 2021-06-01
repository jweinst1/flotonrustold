use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::ptr;
use crate::ptrs::SharedPtr;
use crate::bit_trie::BitTrie;

pub struct KVPair<T>(String, SharedPtr<T>);



/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foo() {
    }
}*/
