use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicI64, Ordering};
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
	AUInt(AtomicU64),
	IInt(i64),
	AIInt(AtomicI64)
}

impl Value {
	#[inline]
	pub fn to_bool(&self) -> bool {
		match self {
			Value::Nothing => false,
			Value::Bool(b) => *b,
			Value::ABool(b) => b.load(Ordering::Acquire),
			Value::UInt(n) => *n != 0,
			Value::AUInt(n) => n.load(Ordering::Acquire) != 0,
			Value::IInt(n) => *n != 0,
			Value::AIInt(n) => n.load(Ordering::Acquire) != 0
		}
	}

	#[inline]
	pub fn to_uint(&self) -> u64 {
		match self {
			Value::Nothing => 0,
			Value::Bool(b) => *b as u64,
			Value::ABool(b) => b.load(Ordering::Acquire) as u64,
			Value::UInt(n) => *n,
			Value::AUInt(n) => n.load(Ordering::Acquire),
			Value::IInt(n) => *n as u64,
			Value::AIInt(n) => n.load(Ordering::Acquire) as u64
		}
	}

	#[inline]
	pub fn to_iint(&self) -> i64 {
		match self {
			Value::Nothing => 0,
			Value::Bool(b) => *b as i64,
			Value::ABool(b) => b.load(Ordering::Acquire) as i64,
			Value::UInt(n) => *n as i64,
			Value::AUInt(n) => n.load(Ordering::Acquire) as i64,
			Value::IInt(n) => *n,
			Value::AIInt(n) => n.load(Ordering::Acquire)
		}
	}

    pub fn fetch_add(&self, other:&Value, order:Ordering, key:*const u64) -> Result<Value, FlotonErr> {
        match self {
            Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
            Value::AUInt(n) => Ok(Value::UInt(n.fetch_add(other.to_uint(), order))),
            Value::AIInt(n) => Ok(Value::IInt(n.fetch_add(other.to_iint(), order))),
            Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::ABool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)) // todo fix to proper error
        }
    }

    pub fn fetch_sub(&self, other:&Value, order:Ordering, key:*const u64) -> Result<Value, FlotonErr> {
        match self {
            Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
            Value::AUInt(n) => Ok(Value::UInt(n.fetch_sub(other.to_uint(), order))),
            Value::AIInt(n) => Ok(Value::IInt(n.fetch_sub(other.to_iint(), order))),
            Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::ABool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)) // todo fix to proper error
        }
    }

	pub fn store(&self, other:&Value, order:Ordering, key:*const u64) -> Result<(), FlotonErr> {
		match self {
			Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
			Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
			Value::ABool(b) => { b.store(other.to_bool(), order); Ok(()) },
			Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
			Value::AUInt(n) => { n.store(other.to_uint(), order); Ok(()) },
			Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_IINT)),
			Value::AIInt(n) => { n.store(other.to_iint(), order); Ok(()) }
		}
	}

	pub fn swap(&self, other:&Value, order:Ordering, key:*const u64) -> Result<Value, FlotonErr> {
		match self {
			Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
			Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
			Value::ABool(b) => Ok(Value::Bool(b.swap(other.to_bool(), order))),
			Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
			Value::AUInt(n) => Ok(Value::UInt(n.swap(other.to_uint(), order))),
			Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_IINT)),
			Value::AIInt(n) => Ok(Value::IInt(n.swap(other.to_iint(), order)))
		}
	}

	pub fn cond_store(&self, 
		              expected:&Value, 
		              desired:&Value, 
		              order:Ordering,
		              key:*const u64) -> Result<bool, FlotonErr> {
		match self {
			Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
			Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
			Value::ABool(b) => match b.compare_exchange(expected.to_bool(), desired.to_bool(), order, Ordering::Relaxed) {
				Ok(_) => Ok(true),
				Err(_) => Ok(false)
			},
			Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
			Value::AUInt(n) => match n.compare_exchange(expected.to_uint(), desired.to_uint(), order, Ordering::Relaxed) {
				Ok(_) => Ok(true),
				Err(_) => Ok(false)
			},
			Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_IINT)),
			Value::AIInt(n) => match n.compare_exchange(expected.to_iint(), desired.to_iint(), order, Ordering::Relaxed) {
				Ok(_) => Ok(true),
				Err(_) => Ok(false)
			}
		}
	}

    pub fn cond_swap(&self, 
                      expected:&Value, 
                      desired:&Value, 
                      order:Ordering,
                      key:*const u64) -> Result<(bool, Value), FlotonErr> {
        match self {
            Value::Nothing => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_NOTHING)),
            Value::Bool(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_BOOL)),
            Value::ABool(b) => match b.compare_exchange(expected.to_bool(), desired.to_bool(), order, Ordering::Relaxed) {
                Ok(v) => Ok((true, Value::Bool(v))),
                Err(v) => Ok((false, Value::Bool(v)))
            },
            Value::UInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_UINT)),
            Value::AUInt(n) => match n.compare_exchange(expected.to_uint(), desired.to_uint(), order, Ordering::Relaxed) {
                Ok(v) => Ok((true, Value::UInt(v))),
                Err(v) => Ok((false, Value::UInt(v)))
            },
            Value::IInt(_) => Err(FlotonErr::TypeNotAtomic(key, constants::VBIN_IINT)),
            Value::AIInt(n) => match n.compare_exchange(expected.to_iint(), desired.to_iint(), order, Ordering::Relaxed) {
                Ok(v) => Ok((true, Value::IInt(v))),
                Err(v) => Ok((false, Value::IInt(v)))
            }
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
			},
			Value::IInt(n) => {
				output.push(constants::VBIN_IINT);
				output.extend_from_slice(&n.to_le_bytes());
			},
			Value::AIInt(n) => {
				output.push(constants::VBIN_AIINT);
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
			},
			constants::VBIN_IINT => {
				let int_val = unsafe { *(input.as_ptr().offset(*place as isize) as *const i64) };
				*place += 8;
				Ok(Value::IInt(int_val))
			},
			constants::VBIN_AIINT => {
				let int_val = unsafe { *(input.as_ptr().offset(*place as isize) as *const i64) };
				*place += 8;
				Ok(Value::AIInt(AtomicI64::new(int_val)))				
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
    fn to_iint_works() {
    	let a = Value::Bool(true);
    	let b = Value::Nothing;
    	let c = Value::IInt(5);
    	let d = Value::AIInt(AtomicI64::new(5));
    	assert_eq!(a.to_iint(), 1);
    	assert_eq!(b.to_iint(), 0);
    	assert_eq!(c.to_iint(), 5);
    	assert_eq!(d.to_iint(), 5);
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
    fn output_iint_works() {
    	let a = Value::IInt(8);
    	let b = Value::AIInt(AtomicI64::new(5));
    	let mut out = Vec::<u8>::new();
    	a.output_binary(&mut out);
    	b.output_binary(&mut out);
    	assert_eq!(out[0], constants::VBIN_IINT);
    	unsafe { assert_eq!(*(out.as_ptr().offset(1 as isize) as *const i64), 8); }
    	assert_eq!(out[9], constants::VBIN_AIINT);
    	unsafe { assert_eq!(*(out.as_ptr().offset(10 as isize) as *const i64), 5); }
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
    fn input_iint_works() {
    	let mut i = 0;
    	let input_num:i64 = -9;
    	let num_bytes = input_num.to_le_bytes();
    	let full_input = [constants::VBIN_IINT, num_bytes[0], num_bytes[1], num_bytes[2], num_bytes[3],
    	                  num_bytes[4], num_bytes[5], num_bytes[6], num_bytes[7]];
    	let res = Value::input_binary(&full_input, &mut i).expect("Could not parse uint value");
    	assert_eq!(i, 9);
    	match res {
    		Value::IInt(n) => assert_eq!(n, -9),
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

    	// Conversion between uint and iint
    	let num = Value::AIInt(AtomicI64::new(-50));
    	let arg = Value::UInt(20);
    	num.store(&arg, Ordering::Release, ptr::null()).expect("Unable to store uint");
    	assert_eq!(20, num.to_uint());
    }

    #[test]
    fn swap_works() {
    	let b = Value::ABool(AtomicBool::new(true));
    	let a = Value::Bool(false);
    	let c = Value::Nothing;
    	match b.swap(&a, Ordering::Release, ptr::null()) {
    		Ok(swapped) => assert!(swapped.to_bool()),
    		Err(e) => panic!("Expected success on bool swap, got err: {:?}", e)
    	}

    	match b.swap(&c, Ordering::Release, ptr::null()) {
    		Ok(swapped) => assert!(!swapped.to_bool()),
    		Err(e) => panic!("Expected success on bool swap, got err: {:?}", e)
    	}

    	let num = Value::AUInt(AtomicU64::new(50));
    	let arg = Value::UInt(30);
    	match num.swap(&arg, Ordering::Release, ptr::null()) {
    		Ok(swapped) => assert_eq!(swapped.to_uint(), 50),
    		Err(e) => panic!("Expected success on uint swap, got err: {:?}", e)
    	}
    }

    #[test]
    fn cond_store_works() {
    	let b = Value::ABool(AtomicBool::new(true));
    	let expected = Value::Bool(true);
    	let desired = Value::Bool(false);
    	match b.cond_store(&expected, &desired, Ordering::Release, ptr::null()) {
    		Ok(res) => assert!(res),
    		Err(e) => panic!("Expected cond store to succeed but got err: {:?}", e)
    	}

    	let num = Value::AUInt(AtomicU64::new(50));
    	let num_expected = Value::UInt(40);
    	let num_desired = Value::UInt(100);
    	match num.cond_store(&num_expected, &num_desired, Ordering::Relaxed, ptr::null()) {
    		Ok(res) => assert!(!res),
    		Err(e) => panic!("Expected cond store to succeed but got err: {:?}", e)
    	}
    }

    #[test]
    fn cond_swap_works() {
        let b = Value::ABool(AtomicBool::new(true));
        let expected = Value::Bool(true);
        let desired = Value::Bool(false);
        match b.cond_swap(&expected, &desired, Ordering::Release, ptr::null()) {
            Ok(pair) => {assert!(pair.0); assert!(pair.1.to_bool());},
            Err(e) => panic!("Expected cond store to succeed but got err: {:?}", e)
        }

        let num = Value::AUInt(AtomicU64::new(50));
        let num_expected = Value::UInt(40);
        let num_desired = Value::UInt(100);
        match num.cond_swap(&num_expected, &num_desired, Ordering::Relaxed, ptr::null()) {
            Ok(pair) => {assert!(!pair.0); assert_eq!(pair.1.to_uint(), 50);},
            Err(e) => panic!("Expected cond store to succeed but got err: {:?}", e)
        }
    }

    #[test]
    fn fetch_add_works() {
        let num = Value::AUInt(AtomicU64::new(0));
        for _ in 0..5 {
            let arg = Value::UInt(1);
            num.fetch_add(&arg, Ordering::Relaxed, ptr::null()).expect("fetch add failed");
        }
        assert_eq!(num.to_uint(), 5);
        let arg2 = Value::UInt(3);
        let prev = num.fetch_add(&arg2, Ordering::Acquire, ptr::null()).unwrap();
        assert_eq!(prev.to_uint(), 5);
        assert_eq!(num.to_uint(), 8);
    }

    #[test]
    fn fetch_sub_works() {
        let num = Value::AUInt(AtomicU64::new(6));
        for _ in 0..5 {
            let arg = Value::UInt(1);
            num.fetch_sub(&arg, Ordering::Relaxed, ptr::null()).expect("fetch add failed");
        }
        assert_eq!(num.to_uint(), 1);
        let arg2 = Value::UInt(1);
        let prev = num.fetch_sub(&arg2, Ordering::Acquire, ptr::null()).unwrap();
        assert_eq!(prev.to_uint(), 1);
    }
}
