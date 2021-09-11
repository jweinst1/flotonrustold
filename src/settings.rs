use crate::traits::*;
use crate::ports::next_port;


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
