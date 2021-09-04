use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::time::Duration;
use crate::threading::{Switch, TVal, ExecUnitGroup};
use crate::traits::*;


struct TcpServer {
	port:u16,
	addr:String,
	core:TcpListener,
	ready:Switch,
	shutter:Switch,
	acceptor:Option<thread::JoinHandle<()>>
}

impl TcpServer {
	pub fn new(init_th_count:usize, th_qsize:usize, addr:String, port:u16, func:fn(*mut TcpStream)) -> TcpServer {
		let ready = Switch::new();
		let rswitch = ready.clone();
		let shut = Switch::new();
		let tshut = shut.clone();
		let mut egroup = ExecUnitGroup::new(init_th_count, th_qsize, func);
		let listener = TcpListener::bind((addr.as_str(), port)).unwrap();
		let tlistener = listener.try_clone().unwrap();
		let handle = thread::spawn(move || {
			while !rswitch.get() {
				thread::park_timeout(Duration::from_millis(500));
			};
			loop {
				if tshut.get() {
					//shutdown logic
					egroup.stop_all();
					break;
				}
				match tlistener.accept() {
					Ok((_socket, addr)) => {
						println!("Got request from {:?}", addr);
						let req = alloc!(_socket);
						match egroup.assign_retried(req, 10, Duration::from_millis(100)) {
							None => {
								// can't handle it
								unsafe {
									req.as_ref().unwrap().shutdown(Shutdown::Both);
								}
								free!(req);
							},
							Some(_) => ()
						}
					},
					Err(e) => println!("Got error from socket {:?}", e)
				}
			}
		});
		TcpServer{
			port:port,
			addr:addr,
			core:listener,
			ready:ready,
			shutter:shut,
			acceptor:Some(handle)
		}
	}

	pub fn start(self) {
		assert!(!self.ready.get());
		self.ready.set(true);
	}

	pub fn stop(&mut self) {
		assert!(self.ready.get());
		assert!(!self.shutter.get());
		self.shutter.set(true);
		self.acceptor.take().unwrap().join().unwrap();
	}
}
