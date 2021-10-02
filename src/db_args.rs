use std::str;
use crate::constants;

#[derive(Debug)]
pub enum ParseResult {
	NoMatch,
	MatchNoArg, // e.g --foo
	Match // either --foo 5 or --foo=5
}

// Used for parsing string argument,
// like --foo=5
#[derive(Debug)]
pub struct ArgRule<'a, T>(&'a str, T);

fn check_arg_rule<T: str::FromStr>(rule:&mut ArgRule<T>, arg:&str) -> ParseResult {
	let bytes = arg.as_bytes();
	let rule_bytes = rule.0.as_bytes();
	let mut j = 0;
	if bytes.len() < rule_bytes.len() {
		return ParseResult::NoMatch;
	}

	for i in 0..rule_bytes.len() {
		if bytes[i] != rule_bytes[i] {
			return ParseResult::NoMatch;
		} else {
			j += 1;
		}
	}

	if j == bytes.len() {
		return ParseResult::MatchNoArg;
	} else if bytes[j] == constants::SSEQ_U8_EQ[0] {
		j += 1;
		if j == bytes.len() {
			// means something like --foo=
			return ParseResult::NoMatch;
		} else {
			unsafe { 
					match str::from_utf8_unchecked(&bytes[j..bytes.len()]).parse::<T>() {
					Ok(v) => {
						rule.1 = v;
						return ParseResult::Match;
					},
					Err(_) => return ParseResult::NoMatch
				}
			}
		}
	} else {
		return ParseResult::NoMatch;
	}
}

// only used after a MatchNoArg
fn check_arg_val<T: str::FromStr>(rule:&mut ArgRule<T>, arg:&str) -> ParseResult {
	match arg.parse::<T>() {
		Ok(v) => { 
			rule.1 = v; 
			ParseResult::Match 
		},
		Err(_) => ParseResult::NoMatch
	}
}

pub fn check_args<T: str::FromStr>(rule:&mut ArgRule<T>, args:&Vec<String>) -> ParseResult {
	let mut state = ParseResult::NoMatch;
	for k in 0..args.len() {
		match state {
			ParseResult::NoMatch => state = check_arg_rule(rule, args[k].as_str()),
			ParseResult::MatchNoArg => state = check_arg_val(rule, args[k].as_str()),
			ParseResult::Match => return state
		}
	}
	state
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_arg_val_works() {
    	let mut rule1 = ArgRule::<bool>("enabled", false);
    	match check_arg_val(&mut rule1, "true") {
    		ParseResult::NoMatch => panic!("Expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::MatchNoArg => panic!("Unexpected result, expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::Match => assert!(rule1.1)
    	}
    }

    #[test]
    fn check_arg_rule_works() {
    	let mut rule1 = ArgRule::<bool>("--enabled", false);
    	match check_arg_rule(&mut rule1, "--enabled=true") {
    		ParseResult::NoMatch => panic!("Expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::MatchNoArg => panic!("Unexpected result, expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::Match => assert!(rule1.1)
    	}
    }

    #[test]
    fn check_args_works() {
    	let mut rule1 = ArgRule::<bool>("--enabled", false);
    	let arguments = vec![String::from("--enabled"), String::from("true"), String::from("--foo")];
    	match check_args(&mut rule1, &arguments) {
    		ParseResult::NoMatch => panic!("Expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::MatchNoArg => panic!("Unexpected result, expected rule: {:?} to be parsed as true", rule1),
    		ParseResult::Match => assert!(rule1.1)	
    	}
    }
}