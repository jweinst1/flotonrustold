use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::io::prelude::*;
use crate::constants;
use crate::errors::FlotonErr;
use crate::traits::*;

#[derive(Debug)]
pub enum Value {
	Nothing,
	Bool(bool),
	ABool(AtomicBool)
}

impl Value {
	#[inline]
	pub fn to_bool(&self) -> bool {
		match self {
			Value::Nothing => false,
			Value::Bool(b) => *b,
			Value::ABool(b) => b.load(Ordering::Acquire)
		}
	}

	pub fn store(&self, other:&Value, key:*const u64) -> Result<(), FlotonErr> {
		match self {
			Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
			Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
			Value::ABool(b) => { b.store(other.to_bool(), Ordering::Release); Ok(()) }
		}
	}
}

impl InPutOutPut for Value {
	fn output_binary(&self, output:&mut Vec<u8>) {
		match self {
			Value::Nothing => output.push(constants::VBIN_NOTHING),
			Value::Bool(b) => {
				output.push(constants::VBIN_BOOL);
				output.push(*b as u8);
			},
			Value::ABool(b) => {
				output.push(constants::VBIN_ABOOL);
				output.push(b.load(Ordering::SeqCst) as u8);
			}
		}
	}

	fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr> {
		match input[*place] {
			constants::VBIN_NOTHING => {
				*place += 1;
				Ok(Value::Nothing)
			},
			constants::VBIN_BOOL => {
				*place += 1;
				let to_ret = Value::Bool(*place != 0);
				*place += 1;
				Ok(to_ret)
			},
			constants::VBIN_ABOOL => {
				*place += 1;
				let to_ret = Value::ABool(AtomicBool::new(*place != 0));
				*place += 1;
				Ok(to_ret)		
			},
			_ => Err(FlotonErr::UnexpectedByte(input[*place]))
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

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
    	assert_eq!(out[0], constants::VBIN_BOOL);
    	assert_eq!(out[1], 1);
    	assert_eq!(out[2], constants::VBIN_ABOOL);
    	assert_eq!(out[3], 1);
    }

    #[test]
    fn store_works() {
    	let b = Value::ABool(AtomicBool::new(true));
    	let a = Value::Bool(false);
    	let c = Value::Nothing;
    	b.store(&a, ptr::null()).expect("Unable to store bool");
    	b.store(&c, ptr::null()).expect("Unable to store bool");
    	assert!(!b.to_bool());
    }
}
