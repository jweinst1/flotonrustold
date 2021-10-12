use std::sync::atomic::Ordering;

use crate::constants::*;
use crate::values::Value;
use crate::errors::FlotonErr;
use crate::traits::*;
use crate::fast_output::out_bool;

/*fn atomic_operation_store(place: &mut usize, cmd:&[u8], key:*const u64, data:&Value, output:&mut Vec<u8>) -> Result<(), FlotonErr> {

}*/


pub fn run_atomic_operation(place: &mut usize, cmd:&[u8], key:*const u64, data:&Value, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
	let op_type = unsafe { *( cmd.as_ptr().offset(*place as isize) as *const u16)};
	*place += 2;
	match op_type {
		OP_ATOMIC_STORE => {
			let arg = match Value::input_binary(cmd, place) {
				Ok(v) => v,
				Err(e) => return Err(e)
			};
			data.store(&arg, Ordering::Release, key)
		},
        OP_ATOMIC_STORE_RELAX => {
            let arg = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            data.store(&arg, Ordering::Relaxed, key)
        },
        OP_ATOMIC_SWAP => {
            let arg = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            // Swap always returns a value
            match data.swap(&arg, Ordering::Release, key) {
                Ok(v) => {v.output_binary(output); Ok(())},
                Err(e) => Err(e)
            }
        },
        OP_ATOMIC_SWAP_RELAX => {
            let arg = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            // Swap always returns a value
            match data.swap(&arg, Ordering::Relaxed, key) {
                Ok(v) => {v.output_binary(output); Ok(())},
                Err(e) => Err(e)
            }
        },
        OP_ATOMIC_COND_STORE => {
            let expected = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            let desired = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            match data.cond_store(&expected, &desired, Ordering::Release, key) {
                Ok(b) => {out_bool(b, output); Ok(())},
                Err(e) => Err(e)
            }
        },
        OP_ATOMIC_COND_STORE_RELAX => {
            let expected = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            let desired = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            match data.cond_store(&expected, &desired, Ordering::Relaxed, key) {
                Ok(b) => {out_bool(b, output); Ok(())},
                Err(e) => Err(e)
            }
        },
        OP_ATOMIC_COND_SWAP => {
            let expected = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            let desired = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            match data.cond_swap(&expected, &desired, Ordering::Release, key) {
                Ok(pair) => {
                    out_bool(pair.0, output);
                    pair.1.output_binary(output); 
                    Ok(())
                },
                Err(e) => Err(e)
            }
        },
        OP_ATOMIC_COND_SWAP_RELAX => {
            let expected = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            let desired = match Value::input_binary(cmd, place) {
                Ok(v) => v,
                Err(e) => return Err(e)
            };
            match data.cond_swap(&expected, &desired, Ordering::Relaxed, key) {
                Ok(pair) => {
                    out_bool(pair.0, output);
                    pair.1.output_binary(output); 
                    Ok(())
                },
                Err(e) => Err(e)
            }
        }
		_ => Err(FlotonErr::UnexpectedByte((op_type >> 8) as u8))
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
        let op_16 = OP_ATOMIC_STORE.to_le_bytes();
    	let cmd = [op_16[0], op_16[1], VBIN_BOOL, 1, /*Unrelated byte*/ 56];
    	let mut output = vec![];
    	let mut i = 0;
    	run_atomic_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run atomic op success");
    	assert_eq!(i, 4);
    	assert!(obj.to_bool());
        i = 0;
        output.clear();
        let op2_16 = OP_ATOMIC_STORE_RELAX.to_le_bytes();
        let cmd2 = [op2_16[0], op2_16[1], VBIN_BOOL, 0, /*Unrelated byte*/ 56];
        run_atomic_operation(&mut i, &cmd2, key.as_ptr(), &obj, &mut output).expect("unable to run atomic op success");
        assert_eq!(i, 4);
        assert!(!obj.to_bool());
    }

    #[test]
    fn atomic_swap_works() {
        let key:[u64;3] = [1, 8, 4455];
        let obj = Value::ABool(AtomicBool::new(false));
        let op_16 = OP_ATOMIC_SWAP.to_le_bytes();
        let cmd = [op_16[0], op_16[1], VBIN_BOOL, 1, /*Unrelated byte*/ 56];
        let mut output = vec![];
        let mut i = 0;
        run_atomic_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run atomic op success");
        assert_eq!(i, 4);
        assert!(obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 0); // swapped out false
        i = 0;
        output.clear();
        let op2_16 = OP_ATOMIC_SWAP_RELAX.to_le_bytes();
        let cmd2 = [op2_16[0], op2_16[1], VBIN_BOOL, 0, /*Unrelated byte*/ 56];
        run_atomic_operation(&mut i, &cmd2, key.as_ptr(), &obj, &mut output).expect("unable to run atomic op success");
        assert_eq!(i, 4);
        assert!(!obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 1); // swapped out true
    }

    #[test]
    fn atomic_cond_store_works() {
        let key:[u64;3] = [1, 8, 4455];
        let obj = Value::ABool(AtomicBool::new(false));
        let op_16 = OP_ATOMIC_COND_STORE.to_le_bytes();
        let cmd = [op_16[0], op_16[1], VBIN_BOOL, 0, VBIN_BOOL, 1, /*Unrelated byte*/ 56];
        let mut output = vec![];
        let mut i = 0;
        run_atomic_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run atomic op success");
        assert_eq!(i, 6);
        assert!(obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 1); // cond store worked
        i = 0;
        output.clear();
        let op2_16 = OP_ATOMIC_COND_STORE_RELAX.to_le_bytes();
        let cmd2 = [op2_16[0], op2_16[1], VBIN_BOOL, 0, VBIN_BOOL, 1, /*Unrelated byte*/ 56];
        run_atomic_operation(&mut i, &cmd2, key.as_ptr(), &obj, &mut output).expect("unable to run atomic op success");
        assert_eq!(i, 6);
        assert!(obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 0); // cond store failed
    }

    #[test]
    fn atomic_cond_swap_works() {
        let key:[u64;3] = [1, 8, 4455];
        let obj = Value::ABool(AtomicBool::new(false));
        let op_16 = OP_ATOMIC_COND_SWAP.to_le_bytes();
        let cmd = [op_16[0], op_16[1], VBIN_BOOL, 0, VBIN_BOOL, 1, /*Unrelated byte*/ 79];
        let mut output = vec![];
        let mut i = 0;
        run_atomic_operation(&mut i, &cmd, key.as_ptr(), &obj, &mut output).expect("Unable to run atomic op success");
        assert_eq!(i, 6);
        assert!(obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 1); // cond swap worked
        assert_eq!(output[2], VBIN_BOOL);
        assert_eq!(output[3], 0); // swapped out value
        i = 0;
        output.clear();
        let op2_16 = OP_ATOMIC_COND_SWAP_RELAX.to_le_bytes();
        let cmd2 = [op2_16[0], op2_16[1], VBIN_BOOL, 0, VBIN_BOOL, 1, /*Unrelated byte*/ 91];
        run_atomic_operation(&mut i, &cmd2, key.as_ptr(), &obj, &mut output).expect("unable to run atomic op success");
        assert_eq!(i, 6);
        assert!(obj.to_bool());
        assert_eq!(output[0], VBIN_BOOL);
        assert_eq!(output[1], 0); // cond swap failed
        assert_eq!(output[2], VBIN_BOOL);
        assert_eq!(output[3], 1); // currently present value
    }
}