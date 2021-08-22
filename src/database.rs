use crate::values::Value;
use crate::containers::Container;
use crate::processors;
use crate::settings::Settings;


struct Database {
	settings:Settings,
	data:Container<Value>
}


impl Database {
	pub fn new(settings:Settings) -> Database {
		Database{settings:settings, data:Container::<Value>::new_map(10)}
	}

	pub fn run(&self, cmd:&[u8], tid:usize, output:&mut Vec<u8>) {
		processors::run_cmd(cmd, self.data, self.settings, tid, output);
	}
}
