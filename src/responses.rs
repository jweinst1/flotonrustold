use std::net::{TcpStream, Shutdown};
use std::sync::atomic::Ordering;
use std::io::prelude::*;
use std::io;
use std::thread;
use crate::ports;
use crate::logging::*;

#[derive(Debug)]
struct ResponseHeader {
	total_size:u32,
	flags:u32
}

#[derive(Debug)]
pub struct Response {
	header:ResponseHeader,
	body:Vec<u8>
}

impl Response {
	pub fn from_vec(output:Vec<u8>, flags:u32) -> Response {
		let hsize = output.len() as u32;
		Response{header:ResponseHeader{total_size:hsize, flags:flags}, body:output}
	}

	pub fn size(&self) -> u32 {
		self.header.total_size
	}

	pub fn to_tcp_stream(&self, stream:&mut TcpStream) -> bool {
		let mut resp_head_buf:[u8;8] = [0;8];
		let size_bytes = self.header.total_size.to_le_bytes();
		let flag_bytes = self.header.flags.to_le_bytes();
		resp_head_buf[0] = size_bytes[0];
		resp_head_buf[1] = size_bytes[1];
		resp_head_buf[2] = size_bytes[2];
		resp_head_buf[3] = size_bytes[3];
		resp_head_buf[4] = flag_bytes[0];
		resp_head_buf[5] = flag_bytes[1];
		resp_head_buf[6] = flag_bytes[2];
		resp_head_buf[7] = flag_bytes[3];
		loop {
			match stream.write_all(&resp_head_buf) {
	    		Ok(_) => break,
	    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
	    		Err(e) => { log_error!(Connections, "Got Error for writing response header: {}", e); return false; }
			}
		}
		if self.header.total_size > 0 {
			loop {
				match stream.write_all(self.body.as_slice()) {
		    		Ok(_) => break,
		    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
		    		Err(e) => { log_error!(Connections, "Got Error for writing response body {}", e); return false; }
				}
			}
		}
		true
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{SocketAddr, TcpListener};
    use std::convert::TryInto;

    #[test]
    fn from_vec_works() {
    	let bytes:Vec<u8> = vec![4, 66, 33, 44];
    	let resp = Response::from_vec(bytes, 0);
    	assert_eq!(resp.size(), 4);
    }

    #[test]
    fn to_tcp_stream_works() {
    	let port_to_use = ports::next_port();
    	let serv_addr = SocketAddr::from(([127, 0, 0, 1], port_to_use));
    	let serv = TcpListener::bind(serv_addr).expect("Could not bind to port");
    	let handle = thcall!({
    		let mut bits:[u8;12] = [0;12];
    		let mut client = TcpStream::connect(serv_addr).expect("Could not connect to port");
    		client.read_exact(&mut bits).expect("Could not read response");
    		assert_eq!(4, u32::from_le_bytes(bits[0..4].try_into().unwrap()));
    		assert_eq!(0, u32::from_le_bytes(bits[4..8].try_into().unwrap()));
    		assert_eq!(bits[8], 44);
    		assert_eq!(bits[9], 33);
    		assert_eq!(bits[10], 22);
    		assert_eq!(bits[11], 11);
    	});
		match serv.accept() {
			Ok((mut socket, addr)) => {
				let bytes:Vec<u8> = vec![44, 33, 22, 11];
				println!("Got request from {:?}", addr);
				let resp = Response::from_vec(bytes, 0);
				assert!(resp.to_tcp_stream(&mut socket));
			},
			Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
				thread::yield_now();
			},
			Err(e) => panic!("Got error from socket {:?}", e)
		}

    	handle.join().unwrap();

    }
}
