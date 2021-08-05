use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::io::prelude::*;
use crate::traits::*;

#[derive(Debug)]
pub enum Value {
	Nothing,
	Bool(bool),
	ABool(AtomicBool)
}

impl Value {
	fn to_bool(&self) -> bool {
		match self {
			Value::Nothing => false,
			Value::Bool(b) => *b,
			Value::ABool(b) => b.load(Ordering::SeqCst)
		}
	}
}

impl OutPut for Value {
	fn output_binary(&self, output:&mut Vec<u8>) {

	}

	fn output_text(&self, output:&mut Vec<u8>) {

	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_bool_works() {
    	let a = Value::Bool(true);
    	let b = Value::Nothing;
    	let c = Value::ABool(AtomicBool::new(true));
    	assert!(a.to_bool());
    	assert!(!b.to_bool());
    	assert!(c.to_bool());
    }
}
