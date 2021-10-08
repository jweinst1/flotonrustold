use std::slice;

/**
 * Functions for processing packed, aligned keys
 */


pub fn key_u64_out_vu8(key:*const u64, output: &mut Vec<u8>) {
	let mut key_ptr = key;
	unsafe {
		let key_depth = *key_ptr;
		output.extend_from_slice(&key_depth.to_le_bytes());
		key_ptr = key_ptr.offset(1 as isize);
		for _ in 0..key_depth {
			let key_len = *key_ptr;
			output.extend_from_slice(&key_len.to_le_bytes());
			key_ptr = key_ptr.offset(1 as isize);
			output.extend_from_slice(slice::from_raw_parts(key_ptr as *const u8, key_len as usize));
			key_ptr = key_ptr.offset((key_len / 8) as isize);
		}
	}
}

pub fn key_u64_len(key:*const u64) -> usize {
	let mut length = 0;
	let mut read_ptr = key;
	// advancement
	let key_depth = unsafe { *read_ptr };
	length += 8;
	read_ptr = unsafe { read_ptr.offset(1 as isize) };
	for _ in 0..key_depth {
		let key_len = unsafe { *read_ptr };
		read_ptr = unsafe { read_ptr.offset((1 + (key_len/8)) as isize) };
		length += (8 + key_len) as usize;
	}
	length
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_u64_out_vu8_works() {
    	let test_data:[u64;9] = [3, 8, 4532, 8, 4478, 16, 5442, 6831, 1];
    	let ptr = test_data.as_ptr();
    	let mut output = Vec::<u8>::new();
    	key_u64_out_vu8(ptr, &mut output);
    	let out_ptr = output.as_ptr();
    	unsafe { assert_eq!(*(out_ptr.offset(0) as *const u64), 3); }
    	unsafe { assert_eq!(*(out_ptr.offset(8) as *const u64), 8); }
    	unsafe { assert_eq!(*(out_ptr.offset(16) as *const u64), 4532); }
    	unsafe { assert_eq!(*(out_ptr.offset(24) as *const u64), 8); }
    	unsafe { assert_eq!(*(out_ptr.offset(32) as *const u64), 4478); }
    	unsafe { assert_eq!(*(out_ptr.offset(40) as *const u64), 16); }
    	unsafe { assert_eq!(*(out_ptr.offset(48) as *const u64), 5442); }
    	unsafe { assert_eq!(*(out_ptr.offset(56) as *const u64), 6831); }
    }

    #[test]
    fn key_u64_len_works() {
    	let test_data:[u64;6] = [2, 8, 4532, 8, 4478, 1];
    	let ptr = test_data.as_ptr();
    	assert_eq!(key_u64_len(ptr), 40);
    }
}