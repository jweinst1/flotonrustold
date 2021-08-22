use crate::constants;
use crate::values::Value;
use crate::tlocal;
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
	let mut not_found = false;
	for _ in 0..key_depth {
		let key_len = cmd[*place] as usize; // 1 byte for now
		*place += 1;
		if !not_found {
			match (*cur_map).get_map(&cmd[*place..(*place + key_len)], tid) {
				Some(inner_map) => cur_map = inner_map,
				None => {
					// start skipping
					not_found = true;
				}
			}
		}
		*place += key_len;
	}
	if !not_found  { 
		(*cur_map).value().output_binary(output); 
	}
}

fn run_cmd_setkv(place: &mut usize, cmd:&[u8], data:&Container<Value>, tid:usize) {
	let key_depth = cmd[*place];
	*place += 1;
	let mut cur_map = data;
	for _ in 0..(key_depth-1) {
		let key_len = cmd[*place] as usize; // 1 byte for now
		*place += 1;
		cur_map = (*cur_map).create_set_map(&cmd[*place..(*place + key_len)], tid, 30 /*todo make specify*/);
		*place += key_len;
	}
	let key_len = cmd[*place] as usize; // 1 byte for now
	*place += 1;
	let harvested_key = &cmd[*place..(*place + key_len)];
	*place += key_len;
	let harvested_val = Container::Val(Value::input_binary(cmd, place));
	(*cur_map).set_map(harvested_key, harvested_val, tid);
}

pub fn run_cmd(cmd:&[u8], data:&Container<Value>, tid:usize, output:&mut Vec<u8>) {
	let mut i = 0;
	loop {
		match cmd[i] {
			constants::CMD_STOP => return,
			constants::CMD_RETURN_KV  => {
				i += 1;
				run_cmd_returnkv(&mut i, cmd, data, output, tid);
			},
			constants::CMD_SET_KV => {
				i += 1;
				run_cmd_setkv(&mut i, cmd, data, tid);
			}
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
    	cmd_buf.push(constants::CMD_RETURN_KV); // cmd code
    	cmd_buf.push(1); // key depth
    	cmd_buf.push(5); // key len
    	write!(cmd_buf, "hello").expect("NO WRITE");
    	cmd_buf.push(constants::CMD_STOP); // stop ops
    	write!(key_buf, "hello").expect("NO WRITE");
    	tlocal::set_epoch();
    	let tid = 0;
    	let val = Value::Bool(false);
    	let cont = Container::<Value>::new_map(50);
    	cont.set_map(key_buf.as_slice(), Container::Val(val), tid);
    	run_cmd(cmd_buf.as_slice(), &cont, tid, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 0);
    }

    #[test]
    fn returnkv_nested_works() {
    	let mut cmd_buf = Vec::<u8>::new();
    	let mut out_buf = Vec::<u8>::new();
    	let mut key_buf = Vec::<u8>::new();
    	cmd_buf.push(constants::CMD_RETURN_KV); // cmd code
    	cmd_buf.push(2); // key depth
    	cmd_buf.push(5); // key len
    	write!(cmd_buf, "hello").expect("NO WRITE");
    	cmd_buf.push(5); // key len
    	write!(cmd_buf, "hello").expect("NO WRITE");
    	cmd_buf.push(constants::CMD_STOP); // stop ops
    	write!(key_buf, "hello").expect("NO WRITE");

    	tlocal::set_epoch();
    	let tid = 0;
    	let val = Value::Bool(false);
    	let cont_inner = Container::<Value>::new_map(10);
    	cont_inner.set_map(key_buf.as_slice(), Container::Val(val), tid);
    	let cont = Container::<Value>::new_map(10);
    	cont.set_map(key_buf.as_slice(), cont_inner, tid);
    	run_cmd(cmd_buf.as_slice(), &cont, tid, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 0);
    }

    #[test]
    fn setkv_works() {
    	tlocal::set_epoch();
    	let cont = Container::<Value>::new_map(50);
    	let tid = 0;
    	// set cmd
    	let mut cmd_s_buf = Vec::<u8>::new();
    	cmd_s_buf.push(constants::CMD_SET_KV);
    	cmd_s_buf.push(1); // key depth
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(constants::VBIN_BOOL); // v type
    	cmd_s_buf.push(1); // v value

    	let mut out_buf = Vec::<u8>::new();
    	// ret cmd
    	cmd_s_buf.push(constants::CMD_RETURN_KV); // cmd ret code
    	cmd_s_buf.push(1); // key depth
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(constants::CMD_STOP); // stop ops
    	run_cmd(cmd_s_buf.as_slice(), &cont, tid, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn setkv_nested_works() {
    	tlocal::set_epoch();
    	let cont = Container::<Value>::new_map(50);
    	let tid = 0;
    	// set cmd
    	let mut cmd_s_buf = Vec::<u8>::new();
    	cmd_s_buf.push(constants::CMD_SET_KV);
    	cmd_s_buf.push(2); // key depth
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(constants::VBIN_BOOL); // v type
    	cmd_s_buf.push(1); // v value

    	let mut out_buf = Vec::<u8>::new();
    	// ret cmd
    	cmd_s_buf.push(constants::CMD_RETURN_KV); // cmd ret code
    	cmd_s_buf.push(2); // key depth
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(5); // key len
    	write!(cmd_s_buf, "hello").expect("NO WRITE");
    	cmd_s_buf.push(constants::CMD_STOP); // stop ops
    	run_cmd(cmd_s_buf.as_slice(), &cont, tid, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 1);
    }
}