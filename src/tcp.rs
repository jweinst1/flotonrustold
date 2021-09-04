use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::{thread, ptr};
use std::net::{TcpListener, TcpStream, Shutdown};
use std::time::Duration;
use std::io;
use crate::threading::{Switch, TVal, ExecUnitGroup, Parker};
use crate::traits::*;
use std::io::prelude::*;


struct TcpServer {
	port:u16,
	addr:String,
	core:TcpListener,
	ready:Switch,
	shutter:Switch,
	acceptor:Option<thread::JoinHandle<()>>
}

impl TcpServer {
	pub fn new(init_th_count:usize, th_qsize:usize, addr:&String, port:u16, mut parker:Parker, func:fn(*mut TcpStream)) -> TcpServer {
		let ready = Switch::new();
		let rswitch = ready.clone();
		let shut = Switch::new();
		let tshut = shut.clone();
		let mut egroup = ExecUnitGroup::new(init_th_count, th_qsize, func);
		let listener = TcpListener::bind((addr.as_str(), port)).unwrap();
		let tlistener = listener.try_clone().unwrap();
		tlistener.set_nonblocking(true).expect("Cannot set non-blocking");
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
								free!(req);
								parker.do_park(false);
							},
							Some(_) => {
								parker.do_park(true);
							}
						}
					},
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
						parker.do_park(false);
					},
					Err(e) => println!("Got error from socket {:?}", e)
				}
			}
		});
		TcpServer{
			port:port,
			addr:addr.clone(),
			core:listener,
			ready:ready,
			shutter:shut,
			acceptor:Some(handle)
		}
	}

	pub fn start(&self) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn do_echo(obj:*mut TcpStream) {
    	unsafe {
    		let mut buf = [0;4];
    		let mut robj = obj.as_ref().unwrap();
    		robj.read(&mut buf);
    		println!("did the read");
    		robj.write(&buf);
    		println!("did the write back");
    		//robj.shutdown(Shutdown::Both);
    	}
    	free!(obj);
    }

    #[test]
    fn echo_works() {
        let serv_addr = String::from("127.0.0.1");
        let serv_port = 8080;
        let pker = Parker::new(5, 200, 15);
        let mut server = TcpServer::new(3, 5, &serv_addr, serv_port, pker, do_echo);
        server.start();
        let mut bits = [0;4];
        let mut resp = [0;4];
        bits[0] = 4;
        bits[1] = 5;
        bits[2] = 88;
        bits[3] = 55;
        let mut sock = TcpStream::connect((serv_addr.as_str(), serv_port)).unwrap();
        sock.write(&bits);
        sock.read(&mut resp);
        assert_eq!(resp[0], bits[0]);
        assert_eq!(resp[1], bits[1]);
        assert_eq!(resp[2], bits[2]);
        assert_eq!(resp[3], bits[3]);
        server.stop();
    }
}
