use crate::traits::*;
use crate::ports::next_port;
use crate::db_args::{check_args, ArgRule};


#[derive(Debug, Clone)]
pub struct Settings {
	pub db_map_slots:usize,
	pub db_port:u16,
	pub conn_th_count:usize,
	pub conn_queue_size:usize,
	pub serv_addr:String,
	pub tcp_park_min:u64,
	pub tcp_park_max:u64,
	pub tcp_park_seg:u64,
	pub th_free_lim:u32
}

impl NewType for Settings {
	fn new() -> Self {
		Settings{db_map_slots:100,
		         db_port:8080,
		         conn_th_count:4,
		         conn_queue_size:50,
		         serv_addr:String::from("127.0.0.1"),
		         tcp_park_min:0,
		         tcp_park_max:1000,
		         tcp_park_seg:50,
		         th_free_lim:5
		     }
	}
}

impl Settings {
	// Tests use unique ports, not specific ones, thus we have to follow
	// that here too
	pub fn set_port_for_testing(&mut self) {
		self.db_port = next_port();
	}

	pub fn from_args(args:&Vec<String>) -> Settings {
		let mut port_rule = ArgRule::<u16>("--port", 8080);
		let mut serv_addr_rule = ArgRule::<String>("--host", String::from("127.0.0.1"));
		let mut conn_th_rule = ArgRule::<usize>("--conn-threads", 4);
		let mut conn_queue_size_rule = ArgRule::<usize>("--conn-queue-size", 50);
		let mut db_map_slots_rule = ArgRule::<usize>("--db-map-slots", 100);
		let mut tcp_park_min_rule = ArgRule::<u64>("--tcp-park-min", 0);
		let mut tcp_park_max_rule = ArgRule::<u64>("--tcp-park-max", 1000);
		let mut tcp_park_seg_rule = ArgRule::<u64>("--tcp-park-seg", 50);
		let mut th_free_lim_rule =  ArgRule::<u32>("--thread-free-limit", 5);

		check_args(&mut port_rule, args);
		check_args(&mut serv_addr_rule, args);
		check_args(&mut conn_th_rule, args);
		check_args(&mut conn_queue_size_rule, args);
		check_args(&mut db_map_slots_rule, args);
		check_args(&mut tcp_park_max_rule, args);
		check_args(&mut tcp_park_min_rule, args);
		check_args(&mut tcp_park_seg_rule, args);
		check_args(&mut th_free_lim_rule, args);

		Settings{
			db_map_slots:db_map_slots_rule.1,
		    db_port:port_rule.1,
		    conn_th_count:conn_th_rule.1,
		    conn_queue_size:conn_queue_size_rule.1,
		    serv_addr:serv_addr_rule.1.clone(),
		    tcp_park_min:tcp_park_min_rule.1,
		    tcp_park_max:tcp_park_max_rule.1,
		    tcp_park_seg:tcp_park_seg_rule.1,
		    th_free_lim:th_free_lim_rule.1
		}
		
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_ports_works() {
    	let mut s1 = Settings::new();
    	let mut s2 = Settings::new();
    	s1.set_port_for_testing();
    	s2.set_port_for_testing();
    	assert!(s1.db_port != s2.db_port);
    }
}
