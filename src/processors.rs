use crate::values::{Value, ValueBinCode};
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
		let key_len = (cmd[*place] as usize); // 1 byte for now
		*place += 1;
		println!("key: {:?}, len: {:?}", &cmd[*place..(*place + key_len)], key_len);
		match (*cur_map).get_map(&cmd[*place..(*place + key_len)], tid) {
			Some(inner_map) => cur_map = inner_map,
			None => {
				*place += key_len;
				return;
			}
		}
		*place += key_len;
	}
	(*cur_map).value().output_binary(output);
}

pub fn run_cmd(cmd:&[u8], data:&Container<Value>, tid:usize, output:&mut Vec<u8>) {
	let mut i = 0;
	loop {
		match cmd[i] {
			0 => return,
			1  => {
				i += 1;
				run_cmd_returnkv(&mut i, cmd, data, output, tid);
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
    	let mut cmd_buf = Vec::<u8>::new();
    	let mut out_buf = Vec::<u8>::new();
    	let mut key_buf = Vec::<u8>::new();
    	cmd_buf.push(1); // cmd code
    	cmd_buf.push(1); // key depth
    	cmd_buf.push(5); // key len
    	write!(cmd_buf, "hello").expect("NO WRITE");
    	cmd_buf.push(0); // stop ops
    	write!(key_buf, "hello").expect("NO WRITE");
    	set_epoch();
    	let tid = 0;
    	let val = Value::Bool(false);
    	let cont = Container::<Value>::new_map(50);
    	cont.set_map(key_buf.as_slice(), Container::Val(val), tid);
    	run_cmd(cmd_buf.as_slice(), &cont, tid, &mut out_buf);
    	assert_eq!(out_buf[0], ValueBinCode::Bool as u8);
    	assert_eq!(out_buf[1], 0);
    }
}