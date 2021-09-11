use crate::containers::Container;
use crate::values::Value;
use crate::processors;
use crate::tcp::{TcpServer, TcpServerStream, TcpServerContext};
use crate::threading::Parker;
use crate::settings::Settings;

struct Database {
	settings:Settings,
	data:Container<Value>,
	server:TcpServer<Database>
}


impl Database {
	fn new(for_testing:bool) -> Database {
		let mut opts = Settings::new();
		if for_testing {
			opts.set_port_for_testing()
		}
		let parker = Parker::new(opts.tcp_park_min, opts.tcp_park_max, opts.tcp_park_seq);
		Database{settings:opts, data:Container::new_map(opts.db_map_slots)}
	}
}