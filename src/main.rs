use std::thread;
use std::process;
use std::env;
use std::time::Duration;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;
use floton::signals;
use floton::logging::*;
use floton::{log_always, alloc, free};
use floton::database::Database;
use floton::settings::Settings;

static THE_DATABASE:AtomicPtr<Database> = AtomicPtr::new(ptr::null_mut());

fn int_handler(sig:i32) {
	log_always!(ShutDown, "Terminating with signal {}", sig);
	unsafe { THE_DATABASE.load(Ordering::SeqCst).as_mut().unwrap().stop(); }
	process::exit(0);
}

fn term_handler(sig:i32) {
	log_always!(ShutDown, "Terminating with signal {}", sig);
	unsafe { THE_DATABASE.load(Ordering::SeqCst).as_mut().unwrap().stop(); }
	process::exit(0);
}

fn main() {
    log_always!(Startup, "---- Floton ----");
    let cli_args = env::args().collect::<Vec<String>>();
    let user_args = &cli_args[1..cli_args.len()];
    log_always!(Startup, "Using cmd line arguments: {:?}", user_args);
    signals::register_int_handler(int_handler);
    signals::register_term_handler(term_handler);

    let settings = Settings::from_args(&cli_args);
    let mut db = Database::new_from_settings(settings);
    db.construct();
    db.start();
    THE_DATABASE.store(alloc!(db), Ordering::SeqCst);
    loop {
    	// todo, main thread
    	thread::park_timeout(Duration::from_millis(5000));
    	log_always!(Main, "Ping");
    }
}
