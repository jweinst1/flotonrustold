use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use crate::shared::*;

pub struct Container<T>(Vec<Shared<T>>);

impl<T> Container<T> {
	pub fn new(size:usize) -> Container<T> {
		let mut fields = vec![];
		fields.reserve(size);
		for _ in 0..size {
			fields.push(Shared::new());
		}
		Container(fields)
	}

	pub fn count(&self) -> usize {
		self.0.len()
	}
}
