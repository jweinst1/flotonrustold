#![allow(dead_code)]
#![allow(unused_macros)]
#[macro_use] mod measure_tools;

use std::env;
use std::process;
use std::time::Instant;

use floton::logging::*;
use floton::log_always;
use floton::traits::*;
use floton::trie::*;

fn int_trie_node() {
	average_s!(IntTrieNodeGet, 100000, {
		let node = IntNode::<u32>::new();
		let _n1 = node.get_seq(5);
		let _n2 = node.get_seq(6);
		let _n3 = node.get_seq(12);
	});
}

const INT_TRIE_NODE_GET:&'static str = "int_trie_node_get";

fn run_bench(key:&str) {
	if key == INT_TRIE_NODE_GET {
		int_trie_node()
	} else {
		log_always!(Bench, "Error: The Benchmark \"{}\" is not found!", key);
		process::exit(2);
	}
}


fn main() {
    let cli_args = env::args().collect::<Vec<String>>();
    let user_args = &cli_args[1..cli_args.len()];
    log_always!(Bench, "Running the benchmarks: {:?}", user_args);
    for i in 0..user_args.len() {
    	run_bench(user_args[i].as_str());
    }
}