
use crate::constants::*;

/**
 * Contains functions to directly output types to binary
 * without first becoming a Value
 */


#[inline]
pub fn out_bool(val:bool, out:&mut Vec<u8>) {
	out.push(VBIN_BOOL);
	out.push(val as u8);
}

#[inline]
pub fn out_u64(val:u64, out:&mut Vec<u8>) {
	out.push(VBIN_UINT);
	out.extend_from_slice(&val.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;

    #[test]
    fn out_bool_works() {
    	let mut output = Vec::<u8>::new();
    	out_bool(true, &mut output);
    	out_bool(false, &mut output);
    	assert_eq!(output[0], VBIN_BOOL);
    	assert_eq!(output[1], 1);
    	assert_eq!(output[2], VBIN_BOOL);
    	assert_eq!(output[3], 0);
    }

    #[test]
    fn out_64_works() {
    	let mut output = Vec::<u8>::new();
    	out_u64(40, &mut output);
    	assert_eq!(output[0], VBIN_UINT);
    	unsafe { assert_eq!(40, u64::from_le_bytes(output[1..9].try_into().expect("Could not convert slice to byte array"))); }
    }
}