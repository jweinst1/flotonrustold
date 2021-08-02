use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub enum Value {
	Nothing,
	Bool(bool),
	ABool(AtomicBool)
}

impl Value {
	fn to_bool(&self) -> bool {
		match self {
			Value::Nothing => false,
			Value::Bool(b) => *b,
			Value::ABool(b) => b.load(Ordering::SeqCst)
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_bool_works() {
    	let a = Value::Bool(true);
    	let b = Value::Nothing;
    	let c = Value::ABool(AtomicBool::new(true));
    	assert!(a.to_bool());
    	assert!(!b.to_bool());
    	assert!(c.to_bool());
    }
}
