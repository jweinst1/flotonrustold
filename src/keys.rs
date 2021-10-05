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

pub fn key_u64_len(key:*const u64) -> isize {
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
	length as isize
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_u64_out_vu8_works() {

    }

    #[test]
    fn key_u64_len_works() {
    	let test_data:[u64;6] = [2, 8, 4532, 8, 4478, 1];
    	let ptr = test_data.as_ptr();
    	assert_eq!(key_u64_len(ptr), 40);
    }
}