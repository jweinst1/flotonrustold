use std::ptr;
use std::sync::atomic::Ordering;
use crate::constants;
use crate::values::Value;
use crate::tlocal;
use crate::containers::Container;
use crate::errors::FlotonErr;
use crate::logging::*;
use crate::traits::*;
use std::io::prelude::*;

/*
 * Run / process commands on containers 
 */


fn run_cmd_returnkv(place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
    let mut key_ptr = unsafe { cmd.as_ptr().offset(*place as isize) as *const u64 };
	let key_depth = unsafe { *key_ptr };
    key_ptr = unsafe { key_ptr.offset(1) };
	*place += 8;
	let mut cur_map = data;
	let mut not_found = false;
	for _ in 0..key_depth {
		let key_len = (unsafe { *key_ptr }) as usize; // todo align error
        key_ptr = unsafe { key_ptr.offset(1) };
		*place += 8;
		if !not_found {
			match (*cur_map).get_map(&cmd[*place..(*place + key_len)]) {
				Some(inner_map) => cur_map = inner_map,
				None => {
					// start skipping
					not_found = true;
				}
			}
		}
        key_ptr = unsafe { key_ptr.offset((key_len / 8) as isize) };
		*place += key_len;
	}
	if !not_found  { 
		(*cur_map).output_binary(output);
        Ok(()) 
	} else {
        Err(FlotonErr::ReturnNotFound(key_ptr))
    }
}

fn run_cmd_setkv(place: &mut usize, cmd:&[u8], data:&Container<Value>) -> Result<(), FlotonErr> {
    let mut key_ptr = unsafe { cmd.as_ptr().offset(*place as isize) as *const u64 };
    let key_depth = unsafe { *key_ptr };
    key_ptr = unsafe { key_ptr.offset(1) };
    *place += 8;
	let mut cur_map = data;
	for _ in 0..(key_depth-1) {
        let key_len = (unsafe { *key_ptr }) as usize;
        key_ptr = unsafe { key_ptr.offset(1) };
        *place += 8;
		cur_map = (*cur_map).create_set_map(&cmd[*place..(*place + key_len)], 30 /*todo make specify*/);
        key_ptr = unsafe { key_ptr.offset((key_len / 8) as isize) };
		*place += key_len;
	}
	let key_len = (unsafe { *key_ptr }) as usize; 
	*place += 8;
    key_ptr = unsafe { key_ptr.offset(1) };
	let harvested_key = &cmd[*place..(*place + key_len)];
	*place += key_len;
    key_ptr = unsafe { key_ptr.offset( (key_len / 8) as isize) };
    match Container::input_binary(cmd, place) {
        Ok(hval) => Ok((*cur_map).set_map(harvested_key, hval)),
        Err(e) => Err(e)
    }
}

pub fn run_cmd(cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) {
	let mut i = 0;
	loop {
		match cmd[i] {
			constants::CMD_STOP => return,
			constants::CMD_RETURN_KV  => {
				i += 1;
				match run_cmd_returnkv(&mut i, cmd, data, output) {
                    Err(e) => e.output_binary(output),
                    Ok(_) => ()
                }
			},
			constants::CMD_SET_KV => {
				i += 1;
				match run_cmd_setkv(&mut i, cmd, data) {
                    Err(e) => { 
                        e.output_binary(output);
                        match e {
                            FlotonErr::UnexpectedByte(b) => {
                                log_error!(Input, "Unexpected set command byre: {}", b);
                                return;
                            },
                            _ => ()
                        } 
                    },
                    Ok(_) => ()
                }
			}
			_ => {
                log_error!(Input, "Unexpected command byte: {}", cmd[i]);
                FlotonErr::UnexpectedByte(cmd[i]).output_binary(output);
                return;
            }
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
    	let val = Value::Bool(false);
    	let cont = Container::<Value>::new_map(50);
    	cont.set_map(key_buf.as_slice(), Container::Val(val));
    	run_cmd(cmd_buf.as_slice(), &cont, &mut out_buf);
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
    	let val = Value::Bool(false);
    	let cont_inner = Container::<Value>::new_map(10);
    	cont_inner.set_map(key_buf.as_slice(), Container::Val(val));
    	let cont = Container::<Value>::new_map(10);
    	cont.set_map(key_buf.as_slice(), cont_inner);
    	run_cmd(cmd_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 0);
    }

    #[test]
    fn setkv_works() {
    	tlocal::set_epoch();
    	let cont = Container::<Value>::new_map(50);
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
    	run_cmd(cmd_s_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn setkv_nested_works() {
    	tlocal::set_epoch();
    	let cont = Container::<Value>::new_map(50);
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
    	run_cmd(cmd_s_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn setkv_map_works() {
        tlocal::set_epoch();
        let cont = Container::<Value>::new_map(10);
        let key1 = [33, 55];
        let keym = [22, 121];
        let cmds = [constants::CMD_SET_KV, 1, 2, key1[0], key1[1], 
                    constants::VBIN_CMAP_BEGIN,
                    constants::CMAPB_KEY, 2, keym[0], keym[1], constants::VBIN_BOOL, 1,
                    constants::VBIN_CMAP_END,
                    constants::CMD_RETURN_KV, 2, 2, key1[0], key1[1], 2, keym[0], keym[1],
                    constants::CMD_STOP];
        let mut out_buf = Vec::<u8>::new();
        run_cmd(&cmds, &cont, &mut out_buf);
        assert_eq!(out_buf[0], constants::VBIN_BOOL);
        assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn ret_not_found_works() {
        tlocal::set_epoch();
        let cont = Container::<Value>::new_map(10);
        let key1 = [33, 55];
        let keym = [22, 121];
        let cmds = [constants::CMD_RETURN_KV, 2, 2, key1[0], key1[1], 2, keym[0], keym[1],
                    constants::CMD_STOP];
        let mut out_buf = Vec::<u8>::new();
        run_cmd(&cmds, &cont, &mut out_buf);
        assert_eq!(out_buf.len(), 9);
        assert_eq!(out_buf[0], constants::VBIN_ERROR);
        assert_eq!(out_buf[1], constants::ERR_RET_NOT_FOUND);
        assert_eq!(out_buf[2], 2);
        assert_eq!(out_buf[3], 2);
        assert_eq!(out_buf[4], key1[0]);
        assert_eq!(out_buf[5], key1[1]);
        assert_eq!(out_buf[6], 2);
        assert_eq!(out_buf[7], keym[0]);
        assert_eq!(out_buf[8], keym[1]);
    }
}