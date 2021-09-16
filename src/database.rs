use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use crate::containers::Container;
use crate::values::Value;
use crate::processors;
use crate::tcp::{TcpServer, TcpServerStream, TcpServerContext};
use crate::threading::Parker;
use crate::settings::Settings;
use crate::requests::Request;
use crate::responses::Response;
use crate::traits::*;

#[derive(Debug)]
pub struct Database {
	settings:Settings,
	data:Container<Value>,
	server:AtomicPtr<TcpServer<Database>>
}

impl DBContext for Database {
	fn get_free_lst_lim(&self) -> u32 {
		self.settings.th_free_lim
	}
}


impl Database {
	fn tcp_handler(obj_ptr:*mut TcpServerStream<Database>) {
		let stream = unsafe { obj_ptr.as_mut().unwrap() };
		let context = stream.get_ctx();
		let mut output:Vec<u8> = vec![];
		let req = Request::parse(&mut stream.0).unwrap();
		processors::run_cmd(&req.body, &context.data, &mut output);
		let resp = Response::from_vec(output, 0);
		resp.to_tcp_stream(&mut stream.0);
	}

	fn new_for_testing() -> Database {
		let mut opts = Settings::new();
		opts.set_port_for_testing();
		let slots_size = opts.db_map_slots;
		Database{settings:opts, 
			     data:Container::new_map(slots_size),
			     server:newptr!()}
	}

	fn construct(&mut self) {
		let parker = Parker::new(self.settings.tcp_park_min, 
			                     self.settings.tcp_park_max, 
			                     self.settings.tcp_park_seg);
		// Can't borrow immutably from mut, so need to clone
		let serv_addr = self.settings.serv_addr.clone();
		let serv = TcpServer::new(self.settings.conn_th_count, 
			     	                      self.settings.conn_queue_size, 
			     	                      &serv_addr, 
			     	                      self.settings.db_port, 
			     	                      &parker,
			     	                      Database::tcp_handler,
			     	                      TcpServerContext::new(self));

		self.server.store(alloc!(serv), Ordering::SeqCst);

	}

	fn is_constructed(&self) -> bool {
		isnull!(self.server.load(Ordering::SeqCst))
	}

	fn start(&self) {
		if !self.is_constructed() {
			panic!("Server was attempted to be started while not being constructed!");
		}
		unsafe { self.server.load(Ordering::SeqCst).as_ref().unwrap().start(); }
	}
}