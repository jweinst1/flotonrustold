use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, AtomicPtr, Ordering};
use std::thread;
use std::net::{TcpListener, TcpStream, Shutdown, ToSocketAddrs};
use std::time::Duration;
use std::process::exit;
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
		let listener = match TcpListener::bind((addr.as_str(), port)) {
			Ok(l) => l,
			Err(_) => {
				log_fatal!(Tcp, "The port {} is already in use.", port);
				exit(1); // todo exit codes
			}
		};
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
					Ok((_socket, client_addr)) => {
						log_trace!(Tcp, "Got connection from {}", client_addr);
						let req = alloc!(TcpServerStream(_socket, tcontext.clone()));
						match egroup.assign_retried(req, 10, Duration::from_millis(100)) {
							None => {
								// can't handle it
								log_warn!(Tcp, "Too busy to handle connection from {}", client_addr);
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

	pub fn is_ready(&self) -> bool {
		self.ready.get()
	}
}

/**
 * Does a send and receive under an u64 le size header protocol
 */
pub fn send_receive<T: ToSocketAddrs>(dest:T, content:&[u8]) -> Result<Vec<u8>, ()> {
	let mut client = match TcpStream::connect(dest) {
		Ok(c) => c,
		Err(_) =>  return Err(())
	};
	let size_bytes = (content.len() as u64).to_le_bytes();
	match client.write_all(&size_bytes) {
		Ok(_) => (),
		Err(_) => return Err(())
	}

	match client.write_all(content) {
		Ok(_) => (),
		Err(_) => return Err(())
	}

	match client.flush() {
		Ok(_) => (),
		Err(_) => return Err(())	
	}

	let mut size_received:[u8;8] = [0;8];
	match client.read_exact(&mut size_received) {
		Ok(_) => (),
		Err(_) => return Err(())
	}
	let size_to_read = u64::from_le_bytes(size_received);
	let mut resp_vec = Vec::<u8>::new();
	resp_vec.resize(size_to_read as usize, 0);
	match client.read_exact(resp_vec.as_mut_slice()) {
		Ok(_) => Ok(resp_vec),
		Err(_) => Err(())
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
        while !server.is_ready() {
        	thread::yield_now();
        }
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

    #[test]
    fn send_receive_works() {
    	logging_test_set(LOG_LEVEL_INFO);
    	let serv_port = next_port();
    	let th_serv_port = serv_port;
		let ready = Switch::new();
		let rswitch = ready.clone();
    	let t1 = thread::spawn(move ||{
    		let mut serv = TcpListener::bind(("127.0.0.1", th_serv_port)).expect("Could not bind to port");
    		rswitch.set(true);
    		match serv.accept() {
    			Ok((mut send_socket, addr)) => {
    				log_info!(TESTsend_receive_works, "Got request from {:?}", addr);
    				let mut size_bytes:[u8;8] = [0;8];
    				send_socket.read_exact(&mut size_bytes).expect("Could not read size");
    				let size_of_req = u64::from_le_bytes(size_bytes);
    				log_info!(TESTsend_receive_works, "Got request size: {}", size_of_req);
    				let mut read_vec = Vec::<u8>::new();
    				read_vec.resize(size_of_req as usize, 0);
    				send_socket.read_exact(read_vec.as_mut_slice()).expect("Could not read req");
    				let send_back_size_bytes = size_of_req.to_le_bytes();
    				send_socket.write_all(&send_back_size_bytes).expect("Could not write back size");
    				send_socket.write_all(&read_vec).expect("Could not write back body");
    				send_socket.flush().expect("Could not flush");
    			},
    			Err(e) => {
    				log_error!(TESTsend_receive_works, "Could not accept request, {:?}", e);
    				panic!("{:?}", e);
    			}
    		}
    	});
    	let to_send = vec![1, 2, 3, 4];
    	loop {
    		if !ready.get() {
    			thread::yield_now();
    		} else {
    			break;
    		}
    	}
    	let result = send_receive(("127.0.0.1", serv_port), &to_send).unwrap();
    	assert_eq!(result.len(), 4);
    	assert_eq!(result[0], to_send[0]);
    	assert_eq!(result[1], to_send[1]);
    	assert_eq!(result[2], to_send[2]);
    	assert_eq!(result[3], to_send[3]);

    	t1.join().unwrap();
    }
}
