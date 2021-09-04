use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use crate::threading::{Switch, ExecUnit};
use crate::dbstates;


struct TcpServer {
	port:u16,
	addr:String,
	core:TcpListener,
	ready:Switch,
	workers:Vec<ExecUnit<TcpStream>>,
	acceptor:Option<JoinHandle<()>>
}

impl TcpServer {
	pub fn new(init_th_count:usize, addr:String, port:u16) -> TcpServer {
		let ready = Switch::new();
		let rswitch = switch.clone();
		let handle = thread::spawn({move ||
			while !rswitch.get() {
				thread::park_timeout(Duration::from_millis(500));
			}
			loop {

			}
		});
		let mut threads = vec![];
		for i in 0..init_th_count {
			threads.push(ExecUnit::new(3, some_handling));
		}
		TcpServer{
			port:port,
			addr:addr,
			core:TcpListener::bind(addr, port),
			ready:rswitch,
			workers:
			accepter:Some(handle)
		}
	}

	pub fn start(self) {
		assert!(!self.ready.get());
		self.ready.set(true);
	}

	pub fn stop(&mut self) {
		// todo
		self.handle.take().unwrap().join().unwrap();
	}
}
