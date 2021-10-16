use std::sync::atomic::Ordering;

use crate::constants::*;
use crate::values::Value;
use crate::shared::{Shared, TimePtr};
use crate::containers::Container;
use crate::errors::FlotonErr;
use crate::traits::*;
use crate::fast_output::{out_bool, out_u64, out_i64};

/**
 * Files that handles normal operations (types can be anything)
 */

pub fn run_normal_operation(place: &mut usize, cmd:&[u8], key:*const u64, data:&Shared<Container<Value>>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
	let op_type = unsafe { *( cmd.as_ptr().offset(*place as isize) as *const u16)};
	*place += 2;
	match op_type {
		OP_NORM_UPDATE => {
            let arg = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            data.write(TimePtr::make(Container::Val(arg)));
            Ok(())
		},
		_ => Err(FlotonErr::UnexpectedByte((op_type >> 8) as u8))
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    #[test]
    fn update_works() {
    	let key:[u64;3] = [1, 8, 4455];
    	let obj = Shared::<Container<Value>>::new();
    	obj.write(TimePtr::make(Container::Val(Value::UInt(0))));
        let op_16 = OP_NORM_UPDATE.to_le_bytes();
    	let cmd = [op_16[0], op_16[1], VBIN_BOOL, 1, /*Unrelated byte*/ 56];
    	let mut output = vec![];
    	let mut i = 0;
    	run_normal_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run normal operation");
    	assert_eq!(i, 4);
    	unsafe { assert!(obj.read().as_ref().unwrap().0.value().unwrap().to_bool()); }
    }
}