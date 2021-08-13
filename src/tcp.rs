use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::net::{TcpListener, TcpStream};
use crate::traits::*;
use crate::threading::*;


struct TcpServer {
	port:u16,
	addr:String,
	core:AtomicPtr<TcpListener>,
	running:AtomicBool
}

impl NewType for TcpServer {
	fn new() -> Self {
		TcpServer{
			port:8080,
			addr:String::from("127.0.0.1"),
			core:AtomicPtr::new(ptr::null_mut()),
			running:AtomicBool::new(false)
		}
	}
}

impl TcpServer {
	pub fn new(addr:String, port:u16) -> TcpServer {
		TcpServer{
			port:port,
			addr:addr,
			core:AtomicPtr::new(ptr::null_mut()),
			running:AtomicBool::new(false)
		}
	}
}
