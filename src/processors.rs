use crate::values::Value;
use crate::epoch::set_epoch;
use crate::containers::Container;
use crate::traits::*;
use std::fmt::Debug;

/*
 * Run / process commands on containers 
 */
#[repr(u8)]
enum CommandCode {
 	EndOps,
 	ReturnKV
}

fn run_cmd_returnkv(place: &mut usize, cmd:&[u8], data:&Container<Value>, output:&mut Vec<u8>) {
	let key_depth = cmd[place];
	place += 1;
	for _ in 0..key_depth {
		let key_len = cmd[place]; // 1 byte for now
		place += 1;
		// to do keys directly as bytes
	}
}

pub fn run_cmd(cmd:&[u8], data:&Container<Value>) -> Vec<u8> {
	let mut i = 0;
	let output = Vec::<u8>::new();
	loop {
		match cmd[i] {
			CommandCode::EndOps => break,
			CommandCode::ReturnKV => {
				i += 1;
				run_cmd_returnkv(&mut i, cmd, data, &mut output);
			},
			_ => panic!("Unexpected cmd byte {:?}", cmd[i])
		}
	}
	output
}