use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::io::prelude::*;
use crate::constants;
use crate::errors::FlotonErr;
use crate::traits::*;

#[derive(Debug)]
pub enum Value {
	Nothing,
	Bool(bool),
	ABool(AtomicBool),
	UInt(u64),
	AUInt(AtomicU64)
}

impl Value {
	#[inline]
	pub fn to_bool(&self) -> bool {
		match self {
			Value::Nothing => false,
			Value::Bool(b) => *b,
			Value::ABool(b) => b.load(Ordering::Acquire),
			Value::UInt(n) => *n != 0,
			Value::AUInt(n) => n.load(Ordering::Acquire) != 0
		}
	}

	#[inline]
	pub fn to_uint(&self) -> u64 {
		match self {
			Value::Nothing => 0,
			Value::Bool(b) => *b as u64,
			Value::ABool(b) => b.load(Ordering::Acquire) as u64,
			Value::UInt(n) => *n,
			Value::AUInt(n) => n.load(Ordering::Acquire)
		}
	}

	pub fn store(&self, other:&Value, order:Ordering, key:*const u64) -> Result<(), FlotonErr> {
		match self {
			Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
			Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
			Value::ABool(b) => { b.store(other.to_bool(), order); Ok(()) },
			Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
			Value::AUInt(n) => { n.store(other.to_uint(), order); Ok(()) }
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
			},
			Value::UInt(n) => {
				output.push(constants::VBIN_UINT);
				output.extend_from_slice(&n.to_le_bytes());
			},
			Value::AUInt(n) => {
				output.push(constants::VBIN_AUINT);
				output.extend_from_slice(&n.load(Ordering::Acquire).to_le_bytes());
			}
		}
	}

	fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr> {
		let in_type = input[*place];
		*place += 1;
		match in_type {
			constants::VBIN_NOTHING => {
				Ok(Value::Nothing)
			},
			constants::VBIN_BOOL => {
				let to_ret = Value::Bool(input[*place] != 0);
				*place += 1;
				Ok(to_ret)
			},
			constants::VBIN_ABOOL => {
				let to_ret = Value::ABool(AtomicBool::new(input[*place] != 0));
				*place += 1;
				Ok(to_ret)		
			},
			constants::VBIN_UINT => {
				let int_val = unsafe { *(input.as_ptr().offset(*place as isize) as *const u64) };
				*place += 8;
				Ok(Value::UInt(int_val))
			},
			constants::VBIN_AUINT => {
				let int_val = unsafe { *(input.as_ptr().offset(*place as isize) as *const u64) };
				*place += 8;
				Ok(Value::AUInt(AtomicU64::new(int_val)))				
			}
			_ => Err(FlotonErr::UnexpectedByte(in_type))
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
    fn to_uint_works() {
    	let a = Value::Bool(true);
    	let b = Value::Nothing;
    	let c = Value::UInt(5);
    	let d = Value::AUInt(AtomicU64::new(5));
    	assert_eq!(a.to_uint(), 1);
    	assert_eq!(b.to_uint(), 0);
    	assert_eq!(c.to_uint(), 5);
    	assert_eq!(d.to_uint(), 5);
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
    fn output_uint_works() {
    	let a = Value::UInt(8);
    	let b = Value::AUInt(AtomicU64::new(5));
    	let mut out = Vec::<u8>::new();
    	a.output_binary(&mut out);
    	b.output_binary(&mut out);
    	assert_eq!(out[0], constants::VBIN_UINT);
    	unsafe { assert_eq!(*(out.as_ptr().offset(1 as isize) as *const u64), 8); }
    	assert_eq!(out[9], constants::VBIN_AUINT);
    	unsafe { assert_eq!(*(out.as_ptr().offset(10 as isize) as *const u64), 5); }
    }

    #[test]
    fn input_bool_works() {
    	let mut i = 0;
    	let f_bytes = [constants::VBIN_BOOL, 0];
    	let t_bytes = [constants::VBIN_BOOL, 1];

    	let res1 = Value::input_binary(&f_bytes, &mut i).expect("Could not parse false value");
    	assert_eq!(i, 2);
    	i = 0;
    	let res2 = Value::input_binary(&t_bytes, &mut i).expect("Could not parse true value");
    	assert_eq!(i, 2);
    	match res1 {
    		Value::Bool(b) => assert!(!b),
    		_ => panic!("Expected bool, got other type")
    	}

    	match res2 {
    		Value::Bool(b) => assert!(b),
    		_ => panic!("Expected bool, got other type")
    	}
    }

    #[test]
    fn input_uint_works() {
    	let mut i = 0;
    	let input_num:u64 = 9;
    	let num_bytes = input_num.to_le_bytes();
    	let full_input = [constants::VBIN_UINT, num_bytes[0], num_bytes[1], num_bytes[2], num_bytes[3],
    	                  num_bytes[4], num_bytes[5], num_bytes[6], num_bytes[7]];
    	let res = Value::input_binary(&full_input, &mut i).expect("Could not parse uint value");
    	assert_eq!(i, 9);
    	match res {
    		Value::UInt(n) => assert_eq!(n, 9),
    		_ => panic!("Expected Uint, got other type")
    	}
    }

    #[test]
    fn store_works() {
    	let b = Value::ABool(AtomicBool::new(true));
    	let a = Value::Bool(false);
    	let c = Value::Nothing;
    	b.store(&a, Ordering::Release, ptr::null()).expect("Unable to store bool");
    	b.store(&c, Ordering::Release, ptr::null()).expect("Unable to store bool");
    	assert!(!b.to_bool());
    }
}
