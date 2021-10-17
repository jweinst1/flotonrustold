

/**
 * Contains types related to coordinating auto scaling behavior
 */


#[derive(Debug)]
pub enum AutoScalePolicy {
	WhenAllFull(usize)
}

