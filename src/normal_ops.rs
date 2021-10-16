use std::sync::atomic::Ordering;

use crate::constants::*;
use crate::values::Value;
use crate::shared::Shared;
use crate::containers::Container;
use crate::errors::FlotonErr;
use crate::traits::*;
use crate::fast_output::{out_bool, out_u64, out_i64};

/**
 * Files that handles normal operations (types can be anything)
 */

pub fn run_normal_operation(place: &mut usize, cmd:&[u8], key:*const u64, data:&Shared<Container<Value>>, output:&mut Vec<u8>) -> Result<(), FlotonErr> {
	Ok(())
}