use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};
use std::ptr;
use crate::containers::Container;
use crate::values::Value;
use crate::processors;
use crate::tcp::{TcpServer, TcpServerStream, TcpServerContext};
use crate::threading::Parker;
use crate::settings::Settings;
use crate::requests::Request;
use crate::responses::Response;
use crate::constants::*;
use crate::tlocal;
use crate::traits::*;
use crate::logging::*;

#[derive(Debug)]
struct DatabaseState(AtomicU8);

impl NewType for DatabaseState {
	fn new() -> Self {
		DatabaseState(AtomicU8::new(DBSTATE_START))
	}
}

impl DatabaseState {
	fn to_ok(&self) -> bool {
		if self.0.load(Ordering::Acquire) == DBSTATE_START {
			self.0.store(DBSTATE_OK, Ordering::Release);
			true
		} else {
			log_fatal!(Database, "Database was not in starting state, it was in state {:?}, thus cannot become ready", self);
			false
		}
	}

	fn to_shutdown(&self) -> bool {
		if self.0.load(Ordering::Acquire) == DBSTATE_OK {
			self.0.store(DBSTATE_SHUTTING_DOWN, Ordering::Release);
			true
		} else {
			log_fatal!(Database, "Database was not in ok state, it was in state {:?}, thus cannot shut down", self);
			false
		}
	}
}

#[derive(Debug)]
pub struct Database {
	settings:Settings,
	data:Container<Value>,
	server:AtomicPtr<TcpServer<Database>>,
	state:DatabaseState
}


impl Database {
	fn tcp_handler(obj_ptr:*mut TcpServerStream<Database>) {
		let cstream = unsafe { obj_ptr.as_ref().unwrap() };
		tlocal::set_db(cstream.get_ptr());
		let context = cstream.get_ctx();
		let tstream = unsafe { obj_ptr.as_mut().unwrap() };
		let mut output:Vec<u8> = vec![];
		let req = Request::parse(&mut tstream.0).unwrap();
		processors::run_cmd(&req.body, &context.data, &mut output);
		let resp = Response::from_vec(output, 0);
		resp.to_tcp_stream(&mut tstream.0);
	}

	fn new_for_testing() -> Database {
		let mut opts = Settings::new();
		opts.set_port_for_testing();
		let slots_size = opts.db_map_slots;
		Database{settings:opts, 
			     data:Container::new_map(slots_size),
			     server:newptr!(),
			     state:DatabaseState::new()}
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
			log_fatal!(Database, "Server was attempted to be started while not being constructed!");
			panic!("Cannot start Database");
		}
		unsafe { self.server.load(Ordering::SeqCst).as_ref().unwrap().start(); }
		self.state.to_ok();
	}

	fn stop(&mut self) {
		if self.state.to_shutdown() {
			let serv_ptr = self.server.load(Ordering::Acquire);
			unsafe { serv_ptr.as_mut().unwrap().stop(); }
			free!(serv_ptr);
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_state_works() {
    	let state = DatabaseState::new();
    	assert!(state.to_ok());
    	assert!(state.to_shutdown());
    }
}