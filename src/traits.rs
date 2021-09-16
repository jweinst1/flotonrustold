
pub trait NewType {
	fn new() -> Self;
}

pub trait InPutOutPut {
	fn output_binary(&self, output: &mut Vec<u8>);
	fn input_binary(input:&[u8], place:&mut usize) -> Self;
	// In the future, more output forms may be supported
}

// This trait is useful for passing a context to a database in multiple areas of the code
// It must alwauys be the case that this context object always outlives whom has access to it.
pub trait DBContext {
	fn get_free_lst_lim(&self) -> u32;
}