use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown};
use std::time::Duration;
use std::io;
use crate::threading::{Switch, TVal, ExecUnitGroup, Parker};
use crate::traits::*;
use crate::ports::next_port;
use crate::logging::*;
use std::io::prelude::*;

/**
 * Used as a Send safe container to ship the context object for
 * the tcp server across threads. Any operations on the context object
 * must be atomic and through a const reference.
 */
#[derive(Debug)]
pub struct TcpServerContext<T>(AtomicPtr<T>);

impl<T> TcpServerContext<T> {
	pub fn new(ptr:*mut T) -> Self {
		TcpServerContext(AtomicPtr::new(ptr))
	}
	#[inline]
	pub fn get(&self) -> *mut T {
		self.0.load(Ordering::Acquire)
	}
}

impl<T> Clone for TcpServerContext<T> {
    fn clone(&self) -> Self {
        TcpServerContext(AtomicPtr::new(self.0.load(Ordering::Acquire)))
    }
}

#[derive(Debug)]
pub struct TcpServerStream<T>(pub TcpStream, TcpServerContext<T> /*Context type*/);

impl<T> TcpServerStream<T> {
	#[inline]
	pub fn get_ctx(&self) -> &T {
		unsafe { self.1.get().as_ref().unwrap() }
	}

	#[inline]
	pub fn get_ptr(&self) -> *mut T {
		self.1.get()
	}
}

#[derive(Debug)]
pub struct TcpServer<T> {
	port:u16,
	addr:String,
	core:TcpListener,
	ready:Switch,
	shutter:Switch,
	acceptor:Option<thread::JoinHandle<()>>,
	context:TcpServerContext<T>
}

impl<T: 'static> TcpServer<T> {
	pub fn new(init_th_count:usize, 
		       th_qsize:usize, 
		       addr:&String, 
		       port:u16, 
		       parker:&Parker, 
		       func:fn(*mut TcpServerStream<T>),
		       context:TcpServerContext<T>) -> TcpServer<T> {
		let ready = Switch::new();
		let rswitch = ready.clone();
		let shut = Switch::new();
		let tshut = shut.clone();
		let mut egroup = ExecUnitGroup::new(init_th_count, th_qsize, func);
		let listener = TcpListener::bind((addr.as_str(), port)).unwrap();
		log_info!(Tcp, "Will listen for connections on port {} at address: {}", port, addr);
		let tlistener = listener.try_clone().unwrap();
		match tlistener.set_nonblocking(true) {
			Err(e) => log_fatal!(Tcp, "Could not set non-blocking mode for tcp server, got {}", e),
			_ => ()
		}
		let tcontext = context.clone();
		let mut tparker = parker.clone();
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
						//println!("Got request from {:?}", addr);
						let req = alloc!(TcpServerStream(_socket, tcontext.clone()));
						match egroup.assign_retried(req, 10, Duration::from_millis(100)) {
							None => {
								// can't handle it
								free!(req);
								tparker.do_park(false);
							},
							Some(_) => {
								tparker.do_park(true);
							}
						}
					},
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
						tparker.do_park(false);
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
			acceptor:Some(handle),
			context:context.clone()
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

    struct Context(u8);

    fn do_echo(obj:*mut TcpServerStream<Context>) {
    	unsafe {
    		let mut buf = [0;4];
    		let robj = obj.as_mut().unwrap();
    		loop {
	    		match robj.0.read_exact(&mut buf) {
	    			Ok(_) => break,
	    			Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::park_timeout(Duration::from_millis(5)),
	    			Err(e) => panic!("Got Error on tcp {:?}", e)
	    		}
    		}

    		loop {
	    		match robj.0.write_all(&buf) {
	    			Ok(_) => break,
	    			Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => thread::park_timeout(Duration::from_millis(5)),
	    			Err(e) => panic!("Got Error on tcp {:?}", e)
	    		}
    		}
    		robj.0.flush().expect("Could not flush");
    		robj.0.shutdown(Shutdown::Both).expect("shutdown call failed");
    	}
    	free!(obj);
    }

    struct Stream(TcpStream);

    impl Stream {
    	fn readwrite(&mut self) {
    		let bits = [4, 2, 88, 44];
    		let mut resp = [0;4];

    		self.0.write_all(&bits).expect("Failed to write bits");
    		self.0.flush().expect("Could not flush");
    		self.0.read(&mut resp).expect("Failed to read response");

	        assert_eq!(resp[0], bits[0]);
	        assert_eq!(resp[1], bits[1]);
	        assert_eq!(resp[2], bits[2]);
	        assert_eq!(resp[3], bits[3]);
    	}
    }

    #[test]
    fn st_echo_works() {
    	logging_test_set(LOG_LEVEL_INFO);
        let serv_addr = String::from("127.0.0.1");
        let serv_port = next_port();
        let pker = Parker::new(5, 200, 15);
        let cxt = alloc!(Context(8));
        let mut server = TcpServer::<Context>::new(3, 5, &serv_addr, serv_port, &pker, do_echo, TcpServerContext::new(cxt));
        server.start();
        let mut bits = [0;4];
        let mut resp = [0;4];
        bits[0] = 4;
        bits[1] = 5;
        bits[2] = 88;
        bits[3] = 55;
        let mut sock = TcpStream::connect((serv_addr.as_str(), serv_port)).unwrap();
        sock.write_all(&bits).expect("Could not do the write");
        sock.read_exact(&mut resp).expect("Could not do the read");

        assert_eq!(resp[0], bits[0]);
        assert_eq!(resp[1], bits[1]);
        assert_eq!(resp[2], bits[2]);
        assert_eq!(resp[3], bits[3]);
        server.stop();
        free!(cxt);
    }

    #[test]
    fn mt_echo_works() {
    	logging_test_set(LOG_LEVEL_INFO);
        let serv_addr = String::from("127.0.0.1");
        let serv_port = next_port();
        let pker = Parker::new(5, 200, 15);
        let cxt = alloc!(Context(8));
        let mut server = TcpServer::<Context>::new(3, 5, &serv_addr, serv_port, &pker, do_echo, TcpServerContext::new(cxt));
        server.start();
        let t1 = thcall!(80, 5, Stream(TcpStream::connect(("127.0.0.1", serv_port)).unwrap()).readwrite());
        let t2 = thcall!(40, 5, Stream(TcpStream::connect(("127.0.0.1", serv_port)).unwrap()).readwrite());
        let t3 = thcall!(40, 5, Stream(TcpStream::connect(("127.0.0.1", serv_port)).unwrap()).readwrite());
        let t4 = thcall!(20, 5, Stream(TcpStream::connect(("127.0.0.1", serv_port)).unwrap()).readwrite());
        let t5 = thcall!(20, 5, Stream(TcpStream::connect(("127.0.0.1", serv_port)).unwrap()).readwrite());
        t1.join().unwrap();
        t2.join().unwrap();
        t3.join().unwrap();
        t4.join().unwrap();
        t5.join().unwrap();
        server.stop();
        free!(cxt);
    }
}
