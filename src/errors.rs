use std::slice;
use std::sync::atomic::Ordering;
use crate::constants::*;
use crate::traits::*;
use crate::logging::*;

/**
 * A generic error type that covers any non-fatal error
 */
 #[derive(Debug)]
pub enum FlotonErr {
	DateTime,
	ReturnNotFound(*const u64),
	UnexpectedByte(u8)
}

impl InPutOutPut for FlotonErr {

	fn output_binary(&self, output: &mut Vec<u8>) {
		output.push(VBIN_ERROR);
		match self {
			FlotonErr::DateTime => output.push(ERR_DATE_TIME),
			FlotonErr::ReturnNotFound(keys) => {
				let mut key_ptr = *keys;
				output.push(ERR_RET_NOT_FOUND);
				unsafe {
					let key_depth = *key_ptr;
					output.extend_from_slice(&key_depth.to_le_bytes());
					key_ptr = key_ptr.offset(1 as isize);
					for _ in 0..key_depth {
						let key_len = *key_ptr;
						output.extend_from_slice(&key_len.to_le_bytes());
						key_ptr = key_ptr.offset(1 as isize);
						unsafe { output.extend_from_slice(slice::from_raw_parts(key_ptr as *const u8, key_len as usize)); }
						key_ptr = key_ptr.offset((key_len / 8) as isize);
					}
				}
			},
			FlotonErr::UnexpectedByte(b) => {
				output.push(ERR_UNEXPECT_BYTE);
				output.push(*b);
			}
		}
	}

	fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr> {
		if input[*place] == VBIN_ERROR {
			*place += 1;
			let err_type = input[*place];
			*place += 1;
			match err_type {
				ERR_DATE_TIME => Ok(FlotonErr::DateTime),
				ERR_RET_NOT_FOUND => unsafe {
					let parsed_ptr = input.as_ptr().offset(*place as isize) as *const u64;
					let mut read_ptr = parsed_ptr;
					// advancement
					let key_depth = unsafe { *read_ptr };
					*place += 8;
					read_ptr = read_ptr.offset(1 as isize);
					for _ in 0..key_depth {
						let key_len = unsafe { *read_ptr };
						read_ptr = read_ptr.offset((1 + key_len) as isize);
						*place += (8 + (key_len * 8)) as usize;
						
					}
					return Ok(FlotonErr::ReturnNotFound(parsed_ptr));
				},
				ERR_UNEXPECT_BYTE => {
					let parsed = FlotonErr::UnexpectedByte(input[*place]);
					*place += 1;
					return Ok(parsed);
				},
				_ => return Err(FlotonErr::UnexpectedByte(err_type))
			}
		} else {
			return Err(FlotonErr::UnexpectedByte(input[*place]));
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn err_out_works() {
    	logging_test_set(LOG_LEVEL_DEBUG);
    	let mut keys = Vec::<u8>::new();
    	let key_depth:u64 = 2;
    	let key_length:u64 = 1;
    	let key_1:u64 = 66;
    	let key_2:u64 = 77;
    	keys.extend_from_slice(&key_depth.to_le_bytes());
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_1.to_le_bytes());
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_2.to_le_bytes());
    	keys.push(3);
    	let err_obj = FlotonErr::ReturnNotFound((&keys).as_ptr() as *const u64);
    	let mut buf = vec![];
    	err_obj.output_binary(&mut buf);
    	log_debug!(TESTerr_out_works, "buf vec is {:?}", buf);
    	assert_eq!(buf.len(), 42);
    	assert_eq!(buf[0], VBIN_ERROR);
    	assert_eq!(buf[1], ERR_RET_NOT_FOUND);
    	assert_eq!(buf[2], keys[0]);
    	assert_eq!(buf[3], keys[1]);
    	assert_eq!(buf[4], keys[2]);
    	assert_eq!(buf[5], keys[3]);
    	assert_eq!(buf[6], keys[4]);
    }

    #[test]
    fn err_in_works() {
    	logging_test_set(LOG_LEVEL_DEBUG);
    	let mut keys = Vec::<u8>::new();
    	keys.push(VBIN_ERROR);
    	keys.push(ERR_RET_NOT_FOUND);

    	let key_depth:u64 = 2;
    	let key_length:u64 = 1;
    	let key_1:u64 = 66;
    	let key_2:u64 = 77;
    	keys.extend_from_slice(&key_depth.to_le_bytes());
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_1.to_le_bytes());
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_2.to_le_bytes());

    	let mut i = 0;
    	let err_obj = FlotonErr::input_binary(keys.as_slice(), &mut i).expect("Cannot parse the error from bytes");
    	assert_eq!(i, 42);
    	match err_obj {
    		FlotonErr::ReturnNotFound(ptr) => unsafe {
    			assert_eq!(*ptr, 2);
    			assert_eq!(*(ptr.offset(1)), 1);
    			assert_eq!(*(ptr.offset(2)), 66);
    			assert_eq!(*(ptr.offset(3)), 1);
    			assert_eq!(*(ptr.offset(4)), 77);			
    		},
    		_=> panic!("Execpted return not found error, but got different error {:?}", err_obj)
    	}
    }
}