use std::net::{TcpStream, Shutdown};
use std::mem;
use std::io::prelude::*;
use crate::traits::*;
use crate::ports::next_local_addr;

#[derive(Debug, Clone)]
struct RequestHeader {
	total_size:u32
}

#[derive(Debug)]
struct Request {
	header:RequestHeader,
	body:Vec<u8>
}

impl NewType for Request {
	fn new() -> Self {
		Request{header:RequestHeader{total_size:0}, body:Vec::<u8>::new()}
	}
}

impl Request {
	fn parse(stream:&mut TcpStream) -> Option<Request> {
		let mut req = Request::new();
		let mut size_buf:[u8;4] = [0;4];
		match stream.read_exact(&mut size_buf) {
			Err(_) => return None,
			_ => (),
		}
		req.header.total_size = u32::from_le_bytes(size_buf);
		req.body.resize(req.header.total_size as usize, 0);
		match stream.read(req.body.as_mut_slice()) {
			Err(_) => return None,
			_ => ()
		}
		Some(req)
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{SocketAddr, TcpListener};

    #[test]
    fn request_parse_works() {
		let addrs = [
		    next_local_addr()
		];
		let mut listener = TcpListener::bind(&addrs[..]).unwrap();
		let mut client = TcpStream::connect(&addrs[..]).unwrap();
		let sizer:u32 = 4;
		let bytes:[u8;4] = [55, 22, 33, 44];
		client.write(&sizer.to_le_bytes()).unwrap();
		client.write(&bytes).unwrap();
		let (mut received, _addr) = listener.accept().unwrap();
		println!("Got req from {:?}", _addr);
		let req = Request::parse(&mut received).unwrap();
		assert_eq!(req.header.total_size, 4);
		assert_eq!(req.body.len(), 4);
		assert_eq!(req.body[0], bytes[0]);
		assert_eq!(req.body[1], bytes[1]);
		assert_eq!(req.body[2], bytes[2]);
		assert_eq!(req.body[3], bytes[3]);
    }
}