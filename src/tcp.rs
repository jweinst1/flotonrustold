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

	pub fn start(&self) {
		assert!(!self.running.load(Ordering::SeqCst));
		self.core.store(Box::into_raw(Box::new(TcpListener::bind((self.addr.as_str(), self.port)).unwrap())), Ordering::SeqCst);
		self.running.store(true, Ordering::SeqCst);
	}

	pub fn stop(&self) {
		assert!(self.running.load(Ordering::SeqCst));
		self.running.store(false, Ordering::SeqCst);
		unsafe {
			drop(Box::from_raw(self.core.load(Ordering::SeqCst)));
		}
	}
}
