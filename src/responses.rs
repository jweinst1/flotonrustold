use std::net::{TcpStream, Shutdown};
use std::io::prelude::*;
use std::io;
use std::thread;

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

    #[test]
    fn from_vec_works() {
    	let bytes:Vec<u8> = vec![4, 66, 33, 44];
    	let resp = Response::from_vec(bytes);
    	assert_eq!(resp.size(), 4);
    }
}
