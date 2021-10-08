use std::ptr;
use std::sync::atomic::Ordering;
use crate::atomic_ops::run_atomic_operation;
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


#[derive(Debug)]
enum KeyAction {
    Return,
    AtomicOp
}

fn run_key_action(action:KeyAction, place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
    let mut key_ptr = unsafe { cmd.as_ptr().offset(*place as isize) as *const u64 };
    let key_orig = key_ptr;
    let key_depth = unsafe { *key_ptr };
    key_ptr = unsafe { key_ptr.offset(1) };
    *place += 8;
    let mut cur_map = data;
    let mut not_found = false;
    //advance to last before end
    for _ in 0..(key_depth-1) {
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
        // take special action at last key segment
        match action {
            KeyAction::Return => {
                let key_len = (unsafe { *key_ptr }) as usize; // todo align error
                key_ptr = unsafe { key_ptr.offset(1) };
                *place += 8;
                return match (*cur_map).get_map(&cmd[*place..(*place + key_len)]) {
                    Some(inner_obj) => { 
                        inner_obj.output_binary(output);
                        *place += key_len;
                        Ok(())
                    },
                    None => {
                        *place += key_len;
                        Err(FlotonErr::ReturnNotFound(key_orig)) 
                    }
                };
            },
            KeyAction::AtomicOp => {
                let key_len = (unsafe { *key_ptr }) as usize; // todo align error
                key_ptr = unsafe { key_ptr.offset(1) };
                *place += 8;
                return match (*cur_map).get_map(&cmd[*place..(*place + key_len)]) {
                    Some(inner_obj) => {
                        *place += key_len;
                        let atomic_val = inner_obj.value();
                        match atomic_val {
                            Ok(v) => {
                                run_atomic_operation(place, cmd, key_orig, v, output)
                            },
                            Err(b) => Err(FlotonErr::TypeNotAtomic(key_orig, b))
                        }
                    },
                    None => { 
                        key_ptr = unsafe { key_ptr.offset((key_len / 8) as isize) };
                        *place += key_len;
                        Err(FlotonErr::ReturnNotFound(key_orig)) 
                    }
                };
            }
        }
    } else {
        let key_len = (unsafe { *key_ptr }) as usize; // todo align error
        *place += 8;
        *place += key_len;
        Err(FlotonErr::ReturnNotFound(key_orig))
    }
}

fn run_cmd_op_atomic(place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
    run_key_action(KeyAction::AtomicOp, place, cmd, data, output)
}


fn run_cmd_returnkv(place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
    run_key_action(KeyAction::Return, place, cmd, data, output)
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
		cur_map = (*cur_map).create_set_map(&cmd[*place..(*place + key_len)], tlocal::get_map_slots());
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
                                log_error!(Input, "Unexpected set command byte: {}", b);
                                return;
                            },
                            _ => ()
                        } 
                    },
                    Ok(_) => ()
                }
			},
            constants::CMD_OP_ATOMIC => {
                i += 1;
                match run_cmd_op_atomic(&mut i, cmd, data, output) {
                    Err(e) => e.output_binary(output),
                    Ok(_) => ()
                }
            },
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
        logging_test_set(LOG_LEVEL_DEBUG);

    	let mut cmd_buf = Vec::<u8>::new();
    	let mut out_buf = Vec::<u8>::new();
    	let mut key_buf = Vec::<u8>::new();
    	cmd_buf.push(constants::CMD_RETURN_KV); // cmd code
        let input_key_depth:u64 = 1;
        let input_key_len:u64 = 16;
        cmd_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_buf.push(constants::CMD_STOP); // stop ops
    	write!(key_buf, "1234567_1234567_").expect("NO WRITE");
    	tlocal::set_epoch();
    	let val = Value::Bool(false);
    	let cont = Container::<Value>::new_map(14);
    	cont.set_map(key_buf.as_slice(), Container::Val(val));
    	run_cmd(cmd_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 0);
    }

    #[test]
    fn returnkv_nested_works() {
        logging_test_set(LOG_LEVEL_DEBUG);
    	let mut cmd_buf = Vec::<u8>::new();
    	let mut out_buf = Vec::<u8>::new();
    	let mut key_buf = Vec::<u8>::new();
    	cmd_buf.push(constants::CMD_RETURN_KV); // cmd code
        let input_key_depth:u64 = 2;
        let input_key_len:u64 = 16;
        cmd_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_buf.extend_from_slice(&input_key_len.to_le_bytes());
        write!(cmd_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_buf.push(constants::CMD_STOP); // stop ops
    	write!(key_buf, "1234567_1234567_").expect("NO WRITE");

    	tlocal::set_epoch();
    	let val = Value::Bool(false);
    	let cont_inner = Container::<Value>::new_map(10);
    	cont_inner.set_map(key_buf.as_slice(), Container::Val(val));
    	let cont = Container::<Value>::new_map(10);
    	cont.set_map(key_buf.as_slice(), cont_inner);
        log_debug!(TESTreturnkv_nested_works, "cmd map test: {:?}", cont);

    	run_cmd(cmd_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 0);
    }

    #[test]
    fn setkv_works() { // current
    	tlocal::set_epoch();
    	let cont = Container::<Value>::new_map(10);
    	// set cmd
    	let mut cmd_s_buf = Vec::<u8>::new();
    	cmd_s_buf.push(constants::CMD_SET_KV);
        let input_key_depth:u64 = 1;
        let input_key_len:u64 = 16;
        cmd_s_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_s_buf.push(constants::VBIN_BOOL); // v type
    	cmd_s_buf.push(1); // v value

    	let mut out_buf = Vec::<u8>::new();
    	// ret cmd
    	cmd_s_buf.push(constants::CMD_RETURN_KV); // cmd ret code
        cmd_s_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
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
        let input_key_depth:u64 = 2; // depth
        let input_key_len:u64 = 16; // len in bytes
        cmd_s_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes()); // key len
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_s_buf.push(constants::VBIN_BOOL); // v type
    	cmd_s_buf.push(1); // v value

    	let mut out_buf = Vec::<u8>::new();
    	// ret cmd
    	cmd_s_buf.push(constants::CMD_RETURN_KV); // cmd ret code
        cmd_s_buf.extend_from_slice(&input_key_depth.to_le_bytes());
        cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes());
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_s_buf.extend_from_slice(&input_key_len.to_le_bytes()); // key len
    	write!(cmd_s_buf, "1234567_1234567_").expect("NO WRITE");
    	cmd_s_buf.push(constants::CMD_STOP); // stop ops
    	run_cmd(cmd_s_buf.as_slice(), &cont, &mut out_buf);
    	assert_eq!(out_buf[0], constants::VBIN_BOOL);
    	assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn setkv_map_works() { 
        tlocal::set_epoch();
        let cont = Container::<Value>::new_map(10);
        let mut cmds = Vec::<u8>::new();
        let key1 = [90, 55, 44, 22, 90, 55, 33, 22];
        let keym = [22, 55, 33, 76, 54, 22, 12, 98];
        let key_depth_one:u64 = 1;
        let key_depth_two:u64 = 2;
        let key_length:u64 = 8;
        cmds.push(constants::CMD_SET_KV);
        cmds.extend_from_slice(&key_depth_one.to_le_bytes());
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&key1);
        cmds.push(constants::VBIN_CMAP_BEGIN);
        cmds.push(constants::CMAPB_KEY);
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&keym);
        cmds.push(constants::VBIN_BOOL);
        cmds.push(1);
        cmds.push(constants::VBIN_CMAP_END);
        cmds.push(constants::CMD_RETURN_KV);
        cmds.extend_from_slice(&key_depth_two.to_le_bytes());
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&key1);
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&keym);       
        cmds.push(constants::CMD_STOP);

        let mut out_buf = Vec::<u8>::new();
        run_cmd(&cmds, &cont, &mut out_buf);
        assert_eq!(out_buf[0], constants::VBIN_BOOL);
        assert_eq!(out_buf[1], 1);
    }

    #[test]
    fn ret_not_found_works() { // current
        tlocal::set_epoch();
        logging_test_set(LOG_LEVEL_DEBUG);

        let cont = Container::<Value>::new_map(10);
        let key1 = [90, 55, 44, 22, 90, 55, 33, 22];
        let keym = [22, 55, 33, 76, 54, 22, 12, 98];
        let key1_n = u64::from_le_bytes(key1);
        let keym_n = u64::from_le_bytes(keym);
        let key_depth_two:u64 = 2;
        let key_length:u64 = 8;
        let mut cmds = Vec::<u8>::new();
        cmds.push(constants::CMD_RETURN_KV);
        cmds.extend_from_slice(&key_depth_two.to_le_bytes());
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&key1);
        cmds.extend_from_slice(&key_length.to_le_bytes());
        cmds.extend_from_slice(&keym);
        cmds.push(constants::CMD_STOP);
        log_debug!(TESTret_not_found_works, "cmds buf {:?}", cmds);

        let mut out_buf = Vec::<u8>::new();
        run_cmd(&cmds, &cont, &mut out_buf);
        log_debug!(TESTret_not_found_works, "out buf: {:?}", out_buf);
        let out_ptr = out_buf.as_ptr();
        assert_eq!(out_buf.len(), 42);
        assert_eq!(out_buf[0], constants::VBIN_ERROR);
        assert_eq!(out_buf[1], constants::ERR_RET_NOT_FOUND);
        unsafe { assert_eq!(*(out_ptr.offset(2) as *const u64), 2); } // depth
        unsafe { assert_eq!(*(out_ptr.offset(10) as *const u64), 8); } // len
        unsafe { assert_eq!(*(out_ptr.offset(18) as *const u64), key1_n); }
        unsafe { assert_eq!(*(out_ptr.offset(26) as *const u64), 8); }
        unsafe { assert_eq!(*(out_ptr.offset(34) as *const u64), keym_n); }
    }
}