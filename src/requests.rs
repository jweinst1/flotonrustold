use std::net::{TcpStream, Shutdown};
use std::sync::atomic::Ordering;
use std::io::prelude::*;
use std::convert::TryInto;
use std::thread;
use std::time::Duration;
use std::io;
use crate::traits::*;
use crate::ports::next_local_addr;
use crate::logging::*;

#[derive(Debug, Clone)]
struct RequestHeader {
	total_size:u32,
	flags:u32
}

#[derive(Debug)]
pub struct Request {
	header:RequestHeader,
	pub body:Vec<u8>
}

impl NewType for Request {
	fn new() -> Self {
		Request{header:RequestHeader{total_size:0, flags:0}, body:Vec::<u8>::new()}
	}
}

impl Request {

	pub fn parse(stream:&mut TcpStream) -> Option<Request> {
		let mut req = Request::new();
		let mut head_buf:[u8;8] = [0;8];
		loop {
			match stream.read_exact(&mut head_buf) {
	    		Ok(_) => break,
	    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
				Err(e) => {
					log_error!(Connections, "Failed to read request header with err: {}", e);
					return None;
				}
			}
		}
		req.header.total_size = u32::from_le_bytes(head_buf[0..4].try_into().unwrap());
		req.header.flags = u32::from_le_bytes(head_buf[4..8].try_into().unwrap());
		req.body.resize(req.header.total_size as usize, 0);
		loop {
			match stream.read_exact(req.body.as_mut_slice()) {
	    		Ok(_) => break,
	    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
				Err(e) => {
					log_error!(Connections, "Failed to read request body with err: {}", e);
					return None;
				}
			}
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
		assert_eq!(req.header.flags, 1);
		assert_eq!(req.header.total_size, 4);
		assert_eq!(req.body.len(), 4);
		assert_eq!(req.body[0], bytes[0]);
		assert_eq!(req.body[1], bytes[1]);
		assert_eq!(req.body[2], bytes[2]);
		assert_eq!(req.body[3], bytes[3]);
    }
}