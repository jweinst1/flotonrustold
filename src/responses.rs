use std::net::{TcpStream, Shutdown};
use std::sync::atomic::Ordering;
use std::io::prelude::*;
use std::io;
use std::thread;
use crate::ports;
use crate::logging::*;

#[derive(Debug)]
struct ResponseHeader {
	total_size:u64
}

#[derive(Debug)]
pub struct Response {
	header:ResponseHeader,
	body:Vec<u8>
}

impl Response {
	pub fn from_vec(output:Vec<u8>) -> Response {
		let hsize = output.len() as u64;
		Response{header:ResponseHeader{total_size:hsize}, body:output}
	}

	pub fn size(&self) -> u64 {
		self.header.total_size
	}

	pub fn to_tcp_stream(&self, stream:&mut TcpStream) -> bool {
		let resp_head_buf = self.header.total_size.to_le_bytes();
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
    	let resp = Response::from_vec(bytes);
    	assert_eq!(resp.size(), 4);
    }

    #[test]
    fn to_tcp_stream_works() {
    	logging_test_set(LOG_LEVEL_TRACE);
    	let port_to_use = ports::next_port();
    	let serv_addr = SocketAddr::from(([127, 0, 0, 1], port_to_use));
    	let serv = TcpListener::bind(serv_addr).expect("Could not bind to port");
    	let handle = thcall!({
    		let mut bits:[u8;12] = [0;12];
    		let mut client = TcpStream::connect(serv_addr).expect("Could not connect to port");
    		client.read_exact(&mut bits).expect("Could not read response");
    		assert_eq!(4, u64::from_le_bytes(bits[0..8].try_into().unwrap()));
    		assert_eq!(bits[8], 44);
    		assert_eq!(bits[9], 33);
    		assert_eq!(bits[10], 22);
    		assert_eq!(bits[11], 11);
    	});
		match serv.accept() {
			Ok((mut socket, addr)) => {
				let bytes:Vec<u8> = vec![44, 33, 22, 11];
				log_debug!(TESTto_tcp_stream_works, "Got request from {:?}", addr);
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
