use std::mem::size_of;


#[derive(Debug)]
struct Buffer {
	data:[u8;256], // todo switch to malloc
	len:usize
}

impl Buffer {
	fn new() -> Buffer {
		Buffer{data:[0;256], len}
	}
}