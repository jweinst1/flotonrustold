
pub trait NewType {
	fn new() -> Self;
}

pub trait OutPut {
	fn output_binary(&self, output: &mut Vec<u8>);
	fn output_text(&self, output:&mut Vec<u8>);
}