
pub trait NewType {
	fn new() -> Self;
}

pub trait InPutOutPut {
	fn output_binary(&self, output: &mut Vec<u8>);
	fn input_binary(input:&[u8], place:&mut usize) -> Self;
	// In the future, more output forms may be supported
}