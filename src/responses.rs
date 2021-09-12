use std::net::{TcpStream, Shutdown};
use std::io::prelude::*;
use std::io;
use std::thread;
use crate::ports;

#[derive(Debug)]
struct ResponseHeader {
	total_size:u32
}

#[derive(Debug)]
pub struct Response {
	header:ResponseHeader,
	body:Vec<u8>
}

impl Response {
	pub fn from_vec(output:Vec<u8>) -> Response {
		let hsize = output.len() as u32;
		Response{header:ResponseHeader{total_size:hsize}, body:output}
	}

	pub fn size(&self) -> u32 {
		self.header.total_size
	}

	pub fn to_tcp_stream(&self, stream:&mut TcpStream) -> bool {
		loop {
			match stream.write_all(&self.header.total_size.to_le_bytes()) {
	    		Ok(_) => break,
	    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
	    		Err(e) => { println!("Got Error on tcp write {:?}", e); return false; }
			}
		}

		loop {
			match stream.write_all(self.body.as_slice()) {
	    		Ok(_) => break,
	    		Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::yield_now(),
	    		Err(e) => { println!("Got Error on tcp write {:?}", e); return false; }
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
    	let resp = Response::from_vec(bytes);
    	assert_eq!(resp.size(), 4);
    }

    #[test]
    fn to_tcp_stream_works() {
    	let port_to_use = ports::next_port();
    	let serv_addr = SocketAddr::from(([127, 0, 0, 1], port_to_use));
    	let serv = TcpListener::bind(serv_addr).expect("Could not bind to port");
    	let handle = thcall!({
    		let mut bits:[u8;8] = [0;8];
    		let mut client = TcpStream::connect(serv_addr).expect("Could not connect to port");
    		client.read_exact(&mut bits).expect("Could not read response");
    		assert_eq!(4, u32::from_le_bytes(bits[0..4].try_into().unwrap()));
    		assert_eq!(bits[4], 44);
    		assert_eq!(bits[5], 33);
    		assert_eq!(bits[6], 22);
    		assert_eq!(bits[7], 11);
    	});
		match serv.accept() {
			Ok((mut socket, addr)) => {
				let bytes:Vec<u8> = vec![44, 33, 22, 11];
				println!("Got request from {:?}", addr);
				let resp = Response::from_vec(bytes);
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
