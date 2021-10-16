use std::slice;
use std::sync::atomic::Ordering;
use crate::constants::*;
use crate::traits::*;
use crate::logging::*;
use crate::keys;

/**
 * A generic error type that covers any non-fatal error
 */
 #[derive(Debug)]
pub enum FlotonErr {
	DateTime,
	ReturnNotFound(*const u64),
	UnexpectedByte(u8),
	TypeNotAtomic(*const u64, u8),
    OperationNoSupport(*const u64, u8, u16)
}

impl InPutOutPut for FlotonErr {

	fn output_binary(&self, output: &mut Vec<u8>) {
		output.push(VBIN_ERROR);
		match self {
			FlotonErr::DateTime => output.push(ERR_DATE_TIME),
			FlotonErr::ReturnNotFound(keys) => {
				output.push(ERR_RET_NOT_FOUND);
				keys::key_u64_out_vu8(*keys, output);
			},
			FlotonErr::UnexpectedByte(b) => {
				output.push(ERR_UNEXPECT_BYTE);
				output.push(*b);
			},
			FlotonErr::TypeNotAtomic(key, t) => {
				output.push(ERR_TYPE_NOT_ATOMIC);
				output.push(*t);
				keys::key_u64_out_vu8(*key, output);
			},
            FlotonErr::OperationNoSupport(key, t, o) => {
                output.push(ERR_OPER_NOT_SUPPORTED);
                output.push(*t);
                output.extend_from_slice(&o.to_le_bytes());
                keys::key_u64_out_vu8(*key, output);
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
					*place += keys::key_u64_len(parsed_ptr);
					return Ok(FlotonErr::ReturnNotFound(parsed_ptr));
				},
				ERR_UNEXPECT_BYTE => {
					let parsed = FlotonErr::UnexpectedByte(input[*place]);
					*place += 1;
					return Ok(parsed);
				},
				ERR_TYPE_NOT_ATOMIC => {
					let val_type = input[*place];
					*place += 1;
					let parsed_ptr = unsafe { input.as_ptr().offset(*place as isize) as *const u64 };
					let parsed = FlotonErr::TypeNotAtomic(parsed_ptr, val_type);
					*place += keys::key_u64_len(parsed_ptr);
					return Ok(parsed);
				},
                ERR_OPER_NOT_SUPPORTED => {
                    let val_type = input[*place];
                    *place += 1;
                    let op_bytes = [input[*place], input[*place + 1]];
                    *place += 2;
                    let parsed_ptr = unsafe { input.as_ptr().offset(*place as isize) as *const u64 };
                    let parsed = FlotonErr::OperationNoSupport(parsed_ptr, val_type, u16::from_le_bytes(op_bytes));
                    *place += keys::key_u64_len(parsed_ptr);
                    return Ok(parsed);
                }
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
    	let key_length:u64 = 8;
    	let key_1 = [33, 55, 44, 123, 221, 71, 81, 91];
    	let key_2 = [33, 25, 44, 223, 121, 71, 81, 91];
    	keys.extend_from_slice(&key_depth.to_le_bytes());
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_1);
    	keys.extend_from_slice(&key_length.to_le_bytes());
    	keys.extend_from_slice(&key_2);
    	keys.push(3);
    	let err_obj = FlotonErr::ReturnNotFound((&keys).as_ptr() as *const u64);
    	let mut buf = vec![];
    	err_obj.output_binary(&mut buf);
    	log_debug!(TESTerr_out_works, "buf vec is {:?}", buf);
    	let out_ptr = buf.as_ptr();
    	assert_eq!(buf.len(), 42);
    	assert_eq!(buf[0], VBIN_ERROR);
    	assert_eq!(buf[1], ERR_RET_NOT_FOUND);
    	unsafe { assert_eq!(*(out_ptr.offset(2) as *const u64), 2); }
    	unsafe { assert_eq!(*(out_ptr.offset(10) as *const u64), 8); }
    	unsafe { assert_eq!(*(out_ptr.offset(18)), 33); }
    	unsafe { assert_eq!(*(out_ptr.offset(19)), 55); }
    	unsafe { assert_eq!(*(out_ptr.offset(20)), 44); }
    	unsafe { assert_eq!(*(out_ptr.offset(21)), 123); }
    	unsafe { assert_eq!(*(out_ptr.offset(22)), 221); }
    	unsafe { assert_eq!(*(out_ptr.offset(23)), 71); }
    	unsafe { assert_eq!(*(out_ptr.offset(24)), 81); }
    	unsafe { assert_eq!(*(out_ptr.offset(25)), 91); }
    	unsafe { assert_eq!(*(out_ptr.offset(26) as *const u64), 8); }
    	unsafe { assert_eq!(*(out_ptr.offset(34)), 33); }
    	unsafe { assert_eq!(*(out_ptr.offset(35)), 25); }
    	unsafe { assert_eq!(*(out_ptr.offset(36)), 44); }
    	unsafe { assert_eq!(*(out_ptr.offset(37)), 223); }
    	unsafe { assert_eq!(*(out_ptr.offset(38)), 121); }
    	unsafe { assert_eq!(*(out_ptr.offset(39)), 71); }
    	unsafe { assert_eq!(*(out_ptr.offset(40)), 81); }
    	unsafe { assert_eq!(*(out_ptr.offset(41)), 91); }
    }

    #[test]
    fn err_in_works() {
    	logging_test_set(LOG_LEVEL_DEBUG);
    	let mut keys = Vec::<u8>::new();
    	keys.push(VBIN_ERROR);
    	keys.push(ERR_RET_NOT_FOUND);

    	let key_depth:u64 = 2;
    	let key_length:u64 = 8;
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
    			assert_eq!(*(ptr.offset(1)), 8);
    			assert_eq!(*(ptr.offset(2)), 66);
    			assert_eq!(*(ptr.offset(3)), 8);
    			assert_eq!(*(ptr.offset(4)), 77);			
    		},
    		_=> panic!("Execpted return not found error, but got different error {:?}", err_obj)
    	}
    }

    #[test]
    fn err_in_type_not_atom_works() {
    	let mut err_data = Vec::<u8>::new();
    	err_data.push(VBIN_ERROR);
    	err_data.push(ERR_TYPE_NOT_ATOMIC);
    	err_data.push(VBIN_BOOL);

    	let key_depth:u64 = 2;
    	let key_length:u64 = 8;
    	let key_1:u64 = 6644;
    	let key_2:u64 = 7722;
    	err_data.extend_from_slice(&key_depth.to_le_bytes());
    	err_data.extend_from_slice(&key_length.to_le_bytes());
    	err_data.extend_from_slice(&key_1.to_le_bytes());
    	err_data.extend_from_slice(&key_length.to_le_bytes());
    	err_data.extend_from_slice(&key_2.to_le_bytes());

    	let mut i = 0;
    	let err_obj = FlotonErr::input_binary(err_data.as_slice(), &mut i).expect("Cannot parse the error from bytes");
    	assert_eq!(i, 43);
    	match err_obj {
    		FlotonErr::TypeNotAtomic(ptr, t) => unsafe {
    			assert_eq!(t, VBIN_BOOL);
    			assert_eq!(*ptr, 2);
    			assert_eq!(*(ptr.offset(1)), 8);
    			assert_eq!(*(ptr.offset(2)), 6644);
    			assert_eq!(*(ptr.offset(3)), 8);
    			assert_eq!(*(ptr.offset(4)), 7722);
    		},
    		_ => panic!("Expected type not atomic error, but got different error {:?}", err_obj)
    	}
    }

    #[test]
    fn err_type_no_support_op_works() {
        let mut err_data = Vec::<u8>::new();
        err_data.push(VBIN_ERROR);
        err_data.push(ERR_OPER_NOT_SUPPORTED);
        err_data.push(VBIN_BOOL);
        err_data.extend_from_slice(&OP_ATOMIC_ADD.to_le_bytes());

        let key_depth:u64 = 2;
        let key_length:u64 = 8;
        let key_1:u64 = 7744;
        let key_2:u64 = 9922;
        err_data.extend_from_slice(&key_depth.to_le_bytes());
        err_data.extend_from_slice(&key_length.to_le_bytes());
        err_data.extend_from_slice(&key_1.to_le_bytes());
        err_data.extend_from_slice(&key_length.to_le_bytes());
        err_data.extend_from_slice(&key_2.to_le_bytes());

        let mut i = 0;
        let err_obj = FlotonErr::input_binary(err_data.as_slice(), &mut i).expect("Cannot parse the error from bytes");
        assert_eq!(i, 45);
        match err_obj {
            FlotonErr::OperationNoSupport(ptr, t, o) => unsafe {
                assert_eq!(t, VBIN_BOOL);
                assert_eq!(o, OP_ATOMIC_ADD);
                assert_eq!(*ptr, 2);
                assert_eq!(*(ptr.offset(1)), 8);
                assert_eq!(*(ptr.offset(2)), 7744);
                assert_eq!(*(ptr.offset(3)), 8);
                assert_eq!(*(ptr.offset(4)), 9922);
            },
            _ => panic!("Expected type operation not supported, but got different error {:?}", err_obj)
        }
    }
}