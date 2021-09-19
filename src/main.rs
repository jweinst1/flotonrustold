use std::thread;
use std::process;
use floton::tlocal;
use floton::datetime;
use floton::signals;

fn int_handler(sig:i32) {
	println!("Terminating with signal {}", sig);
	process::exit(0);
}

fn main() {
    println!("---- Floton DB ----");
    println!("TID: {}", tlocal::tid());
    println!("Unix Time: {}", datetime::unix_time() as u64);
    signals::register_int_handler(int_handler);
    thread::park();
}
