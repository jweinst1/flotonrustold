use crate::constants::*;
use crate::values::Value;
use crate::errors::FlotonErr;
use crate::traits::*;

/*fn atomic_operation_store(place: &mut usize, cmd:&[u8], key:*const u64, data:&Value, output:&mut Vec<u8>) -> Result<(), FlotonErr> {

}*/


pub fn run_atomic_operation(place: &mut usize, cmd:&[u8], key:*const u64, data:&Value, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
	let op_type = cmd[*place];
	*place += 1;
	match op_type {
		OP_ATOMIC_STORE => {
			let arg = match Value::input_binary(cmd, place) {
				Ok(v) => v,
				Err(e) => return Err(e)
			};
			data.store(&arg, key)
		},
		_ => Err(FlotonErr::UnexpectedByte(op_type))
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn atomic_store_works() {
    	let key:[u64;3] = [1, 8, 4455];
    	let obj = Value::ABool(AtomicBool::new(false));
    	let cmd = [OP_ATOMIC_STORE, VBIN_BOOL, 1, /*Unrelated byte*/ 56];
    	let mut output = vec![];
    	let mut i = 0;
    	run_atomic_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run atomic op success");
    	assert_eq!(i, 3);
    	assert!(obj.to_bool());
    }
}