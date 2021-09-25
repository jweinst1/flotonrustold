use std::sync::atomic::{AtomicU16, Ordering};
use std::net::{ToSocketAddrs, SocketAddr, TcpListener};

const PORT_NUMBER_BEGIN:u16 = 10000;
static PORT_NUMBER:AtomicU16 = AtomicU16::new(PORT_NUMBER_BEGIN);



pub fn next_port() -> u16 {
	let port = PORT_NUMBER.fetch_add(1, Ordering::SeqCst);
	if port < PORT_NUMBER_BEGIN {
		// Safely reset port if it goes over
		 match PORT_NUMBER.compare_exchange(port, PORT_NUMBER_BEGIN, Ordering::SeqCst, Ordering::SeqCst) {
		 	Ok(n) => return n,
		 	Err(n) => return n
		}
	}
	port
}

pub fn next_local_addr() -> SocketAddr {
	SocketAddr::from(([127, 0, 0, 1], next_port()))
}

