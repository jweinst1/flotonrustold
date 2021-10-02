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
		let resp = Response::from_vec(output);
		resp.to_tcp_stream(&mut tstream.0);
	}

	pub fn get_free_lim(&self) -> u32 {
		self.settings.th_free_lim
	}

	pub fn new_for_testing() -> Database {
		let mut opts = Settings::new();
		opts.set_port_for_testing();
		Database::new_from_settings(opts)
	}

	pub fn new_from_settings(settings:Settings) -> Database {
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
    	logging_test_set(LOG_LEVEL_DEBUG);
    	tlocal::set_epoch();
    	let mut resp_header:[u8;8] = [0;8];
        let key1 = [33, 55, 44, 123, 221, 71, 81, 91];
        let u64_val_one:u64 = 1;
        let u64_val_8:u64 = 8;
        let mut set_cmd = Vec::<u8>::new();
        set_cmd.push(CMD_SET_KV);
        set_cmd.extend_from_slice(&u64_val_one.to_le_bytes());
        set_cmd.extend_from_slice(&u64_val_8.to_le_bytes());
        set_cmd.extend_from_slice(&key1);
        set_cmd.push(VBIN_BOOL);
        set_cmd.push(1);
        set_cmd.push(CMD_STOP);
        let set_cmd_size = (set_cmd.len() as u64).to_le_bytes();
        // insert size at beginning
        for _ in 0..8 {
        	set_cmd.insert(0, 0);
        }
        set_cmd[0] = set_cmd_size[0];
        set_cmd[1] = set_cmd_size[1];
        set_cmd[2] = set_cmd_size[2];
        set_cmd[3] = set_cmd_size[3];
        set_cmd[4] = set_cmd_size[4];
        set_cmd[5] = set_cmd_size[5];
        set_cmd[6] = set_cmd_size[6];
        set_cmd[7] = set_cmd_size[7];

        let mut get_cmd = Vec::<u8>::new();
        get_cmd.push(CMD_RETURN_KV);
        get_cmd.extend_from_slice(&u64_val_one.to_le_bytes());
        get_cmd.extend_from_slice(&u64_val_8.to_le_bytes());
        get_cmd.extend_from_slice(&key1);
        get_cmd.push(CMD_STOP);

        let get_cmd_size = (get_cmd.len() as u64).to_le_bytes();
        // insert size at beginning
        for _ in 0..8 {
        	get_cmd.insert(0, 0);
        }
        get_cmd[0] = get_cmd_size[0];
        get_cmd[1] = get_cmd_size[1];
        get_cmd[2] = get_cmd_size[2];
        get_cmd[3] = get_cmd_size[3];
        get_cmd[4] = get_cmd_size[4];
        get_cmd[5] = get_cmd_size[5];
        get_cmd[6] = get_cmd_size[6];
        get_cmd[7] = get_cmd_size[7];

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
        let get_resp_size = u64::from_le_bytes(resp_header);
        assert_eq!(get_resp_size, 2);
        let mut resp_body:[u8;2] = [0;2];
        client2.read_exact(&mut resp_body);
        assert_eq!(VBIN_BOOL, resp_body[0]);
        assert_eq!(1, resp_body[1]);
        db.stop();
    }
}