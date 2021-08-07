use crate::values::Value;
use crate::epoch::set_epoch;
use crate::containers::Container;
use crate::traits::*;
use std::io::prelude::*;
use std::fmt::Debug;

/*
 * Run / process commands on containers 
 */


fn run_cmd_returnkv(place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>, tid:usize) {
	let key_depth = cmd[*place];
	*place += 1;
	let mut cur_map = data;
	for _ in 0..key_depth {
		let key_len = cmd[*place] as usize; // 1 byte for now
		*place += 1;
		match (*cur_map).get_map(&cmd[*place..key_len], tid) {
			Some(inner_map) => cur_map = inner_map,
			None => return
		}
		*place += key_len;
	}
	(*cur_map).value().output_binary(output);
}

pub fn run_cmd(cmd:&[u8], data:&Container<Value>, tid:usize) -> Vec<u8> {
	let mut i = 0;
	let mut output = Vec::<u8>::new();
	loop {
		match cmd[i] {
			0 => return output,
			1  => {
				i += 1;
				run_cmd_returnkv(&mut i, cmd, data, &mut output, tid);
			},
			_ => panic!("Unexpected cmd byte {:?}", cmd[i])
		}
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returnkv_works() {
    	let cmd_buf = Vec::<u8>::new();
    	set_epoch();
    	let tid = 0;
    	let key = [22, 33, 31, 4];
    	let val = Value::Bool(false);
    	let cont = Container::<Value>::new_map(50);
    	cont.set_map(&key[..], Container::Val(val), tid);
    	let out = run_cmd()
    }
}