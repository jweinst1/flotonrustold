use crate::constants::*;
use crate::traits::*;

/**
 * A generic error type that covers any non-fatal error
 */
 #[derive(Debug)]
pub enum FlotonErr {
	DateTime,
	ReturnNotFound(*const u8),
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
					output.push(key_depth);
					key_ptr = key_ptr.offset(1);
					for _ in 0..key_depth {
						let key_len = *key_ptr;
						output.push(key_len);
						key_ptr = key_ptr.offset(1);
						for _ in 0..key_len {
							output.push(*key_ptr);
							key_ptr = key_ptr.offset(1);
						}
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
					let parsed = FlotonErr::ReturnNotFound(input.as_ptr().offset(*place as isize));
					// advancement
					let key_depth = input[*place];
					*place += 1;
					for _ in 0..key_depth {
						let key_len = input[*place];
						*place += 1;
						for _ in 0..key_len {
							*place += 1;
						}
					}
					return Ok(parsed);
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
    	let keys = [2, // depth
    	            1, // len
    	            66,
    	            1, // len
    	            77,
    	            3 // some value
    	            ];
    	let err_obj = FlotonErr::ReturnNotFound((&keys).as_ptr());
    	let mut buf = vec![];
    	err_obj.output_binary(&mut buf);
    	assert_eq!(buf.len(), 7);
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
    	let keys = [VBIN_ERROR, ERR_RET_NOT_FOUND, 2, // depth
    	            1, // len
    	            66,
    	            1, // len
    	            77
    	            ];
    	let mut i = 0;
    	let err_obj = FlotonErr::input_binary(&keys, &mut i).expect("Cannot parse the error from bytes");
    	assert_eq!(i, 7);
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