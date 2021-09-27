use crate::errors::FlotonErr;

pub trait NewType {
	fn new() -> Self;
}

pub trait KeyLike {
	fn depth(&self) -> usize;
}

pub trait InPutOutPut {
	fn output_binary(&self, output: &mut Vec<u8>);
	fn input_binary(input:&[u8], place:&mut usize) -> Result<Self, FlotonErr> where Self: Sized;
}