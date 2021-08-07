
pub trait NewType {
	fn new() -> Self;
}

pub trait OutPut {
	fn output_binary(&self, output: &mut Vec<u8>);
	// In the future, more output forms may be supported
}