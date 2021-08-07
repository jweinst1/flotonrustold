use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::io::prelude::*;
use crate::traits::*;


#[repr(u8)]
enum ValueBinCode {
	Nothing = 0,
	Bool = 1,
	ABool = 2
}

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
		match self {
			Value::Nothing => output.push(ValueBinCode::Nothing as u8),
			Value::Bool(b) => {
				output.push(ValueBinCode::Bool as u8);
				output.push(*b as u8);
			},
			Value::ABool(b) => {
				output.push(ValueBinCode::ABool as u8);
				output.push(b.load(Ordering::SeqCst) as u8);
			}
		}
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

    #[test]
    fn output_bool_works() {
    	let a = Value::Bool(true);
    	let b = Value::ABool(AtomicBool::new(true));
    	let mut out = Vec::<u8>::new();
    	a.output_binary(&mut out);
    	b.output_binary(&mut out);
    	assert_eq!(out[0], ValueBinCode::Bool as u8);
    	assert_eq!(out[1], 1);
    	assert_eq!(out[2], ValueBinCode::ABool as u8);
    	assert_eq!(out[3], 1);
    }
}
