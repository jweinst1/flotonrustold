use std::net::{TcpStream, Shutdown};
use std::io::prelude::*;
use std::convert::TryInto;
use crate::traits::*;
use crate::ports::next_local_addr;

#[derive(Debug, Clone)]
struct RequestHeader {
	total_size:u32,
	flags:u32
}

impl RequestHeader {
	fn is_shutdown_req(&self) -> bool {
		(self.flags & 1) == 1
	}
}

#[derive(Debug)]
struct Request {
	header:RequestHeader,
	body:Vec<u8>
}

impl NewType for Request {
	fn new() -> Self {
		Request{header:RequestHeader{total_size:0, flags:0}, body:Vec::<u8>::new()}
	}
}

impl Request {
	fn parse(stream:&mut TcpStream) -> Option<Request> {
		let mut req = Request::new();
		let mut head_buf:[u8;8] = [0;8];
		match stream.read_exact(&mut head_buf) {
			Err(_) => return None,
			_ => (),
		}
		req.header.total_size = u32::from_le_bytes(head_buf[0..4].try_into().unwrap());
		req.header.flags = u32::from_le_bytes(head_buf[4..8].try_into().unwrap());
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
		let listener = TcpListener::bind(&addrs[..]).unwrap();
		let mut client = TcpStream::connect(&addrs[..]).unwrap();
		let sizer:u32 = 4;
		let flags:u32 = 1;
		let bytes:[u8;4] = [55, 22, 33, 44];
		client.write(&sizer.to_le_bytes()).unwrap();
		client.write(&flags.to_le_bytes()).unwrap();
		client.write(&bytes).unwrap();
		let (mut received, _addr) = listener.accept().unwrap();
		println!("Got req from {:?}", _addr);
		let req = Request::parse(&mut received).unwrap();
		assert!(req.header.is_shutdown_req());
		assert_eq!(req.header.total_size, 4);
		assert_eq!(req.body.len(), 4);
		assert_eq!(req.body[0], bytes[0]);
		assert_eq!(req.body[1], bytes[1]);
		assert_eq!(req.body[2], bytes[2]);
		assert_eq!(req.body[3], bytes[3]);
    }
}