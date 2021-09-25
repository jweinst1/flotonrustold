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
		Database::new_from_settings(opts)
	}

	fn new_from_settings(settings:Settings) -> Database {
		let slots_size = settings.db_map_slots;
		Database{settings:settings, 
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
		nonull!(self.server.load(Ordering::SeqCst))
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
    use std::net::TcpStream;
    use std::io::prelude::*;
    use std::convert::TryInto;

    #[test]
    fn db_state_works() {
    	let state = DatabaseState::new();
    	assert!(state.to_ok());
    	assert!(state.to_shutdown());
    }

    #[test]
    fn basic_set_get_works() {
    	logging_test_set(LOG_LEVEL_TRACE);
    	tlocal::set_epoch();
    	let mut resp_header:[u8;8] = [0;8];
        let key1 = [33, 55];
        let set_value = [VBIN_BOOL, 1];
        let set_key1 = [CMD_SET_KV, 1, 2, key1[0], key1[1], set_value[0], set_value[1], CMD_STOP];
        let get_key1 = [CMD_RETURN_KV, 1, 2, key1[0], key1[1], CMD_STOP];
        let set_key1_size:u32 =  8;
        let get_key1_size:u32 =  6;
        let set_sbytes = set_key1_size.to_le_bytes();
        let get_sbytes = get_key1_size.to_le_bytes();
        let set_cmd = [set_sbytes[0], set_sbytes[1], set_sbytes[2], set_sbytes[3], // size
                       0, 0, 0, 0, // flags
                       set_key1[0], set_key1[1], set_key1[2], set_key1[3], set_key1[4], set_key1[5], set_key1[6], set_key1[7]];

        let get_cmd = [get_sbytes[0], get_sbytes[1], get_sbytes[2], get_sbytes[3], // size
                       0, 0, 0, 0, // flags
                       get_key1[0], get_key1[1], get_key1[2], get_key1[3], get_key1[4], get_key1[5]];
        let mut db = Database::new_for_testing();
        log_debug!(TESTbasic_set_get_works, "Set Req: {:?}", set_cmd);
        log_debug!(TESTbasic_set_get_works, "Get Req: {:?}", get_cmd);
        db.construct();
        db.start();
        let mut client = TcpStream::connect((db.settings.serv_addr.as_str(), db.settings.db_port)).expect("Could not connect to db port and addr");
        log_trace!(TESTbasic_set_get_works, "Connecting from {}, to {}", 
        	                             client.local_addr().unwrap(), 
        	                             client.peer_addr().unwrap());
        client.write_all(&set_cmd).expect("Could not write the set request");
        client.read_exact(&mut resp_header).expect("Could not read back from set response");
        // Should, on success, be no resp back.
        assert_eq!(resp_header[0], 0);
        assert_eq!(resp_header[1], 0);
        assert_eq!(resp_header[2], 0);
        assert_eq!(resp_header[3], 0);
        assert_eq!(resp_header[4], 0);
        assert_eq!(resp_header[5], 0);
        assert_eq!(resp_header[6], 0);
        assert_eq!(resp_header[7], 0);

        let mut client2 = TcpStream::connect((db.settings.serv_addr.as_str(), db.settings.db_port)).expect("Could not connect to db port and addr");
        client2.write_all(&get_cmd).expect("Could not write the get request");
        client2.read_exact(&mut resp_header).expect("Could not read back from get resp header");
        let get_resp_size = u32::from_le_bytes(resp_header[0..4].try_into().expect("Could not get size from slice"));
        assert_eq!(get_resp_size, 2);
        let mut resp_body:[u8;2] = [0;2];
        client2.read_exact(&mut resp_body);
        assert_eq!(set_value[0], resp_body[0]);
        assert_eq!(set_value[1], resp_body[1]);
        db.stop();
    }
}