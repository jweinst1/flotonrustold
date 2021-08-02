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
 	ReturnKV
 }

pub fn run_cmd(cmd:&[u8], data:&Container<Value>) {
	for i in 0..cmd.len() {
		match cmd[i] {
			CommandCode::ReturnKV => {},
			_ => panic!("Unexpected cmd  byte {:?}", cmd[i])
		}
	}
}