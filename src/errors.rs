use crate::constants::*;
use crate::traits::*;

/**
 * A generic error type that covers any non-fatal error
 */
 #[derive(Debug)]
pub enum FlotonErr {
	DateTime,
	ReturnNotFound(*const u8)
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
			}
		}
	}

	fn input_binary(input:&[u8], place:&mut usize) -> Self {
		if input[*place] == VBIN_ERROR {
			*place += 1;
			let err_type = input[*place];
			*place += 1;
			match err_type {
				ERR_DATE_TIME => FlotonErr::DateTime,
				ERR_RET_NOT_FOUND => unsafe {  FlotonErr::ReturnNotFound(input.as_ptr().offset(*place as isize)) },
				_ => panic!("Unexpected byte for err type :{}", err_type)
			}
		} else {
			panic!("Unrecognized byte for input_binary on FlotonErr: {}", input[*place]);
		}
	}
}