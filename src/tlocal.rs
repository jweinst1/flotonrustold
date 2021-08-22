use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::thread;
use std::time::Instant;
use std::mem::MaybeUninit;
use std::convert::TryFrom;

static THREAD_CNTR:AtomicUsize = AtomicUsize::new(0);

thread_local!(static TH_ID:usize = THREAD_CNTR.fetch_add(1, Ordering::SeqCst));

fn tid() -> usize {
    TH_ID.with(|x| { *x })
}

static mut MONOTONIC_EPOCH:MaybeUninit<Instant> = MaybeUninit::<Instant>::uninit();
static MONOTONIC_EPOCH_LOCK:AtomicBool = AtomicBool::new(false);
static MONOTONIC_EPOCH_STATE:AtomicBool = AtomicBool::new(false);

// Thread safe way to set epoch
pub fn set_epoch() {
	match MONOTONIC_EPOCH_LOCK.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
		Ok(_) => unsafe {
        	MONOTONIC_EPOCH.as_mut_ptr().write(Instant::now());
        	MONOTONIC_EPOCH_STATE.store(true, Ordering::SeqCst);
    	},
    	Err(_) => while MONOTONIC_EPOCH_LOCK.load(Ordering::SeqCst) {
    		if MONOTONIC_EPOCH_STATE.load(Ordering::SeqCst) {
    			break;
    		} else {
    			thread::yield_now();
    		}
    	}
	}
}

thread_local!(static TH_EPOCH:Instant = unsafe { MONOTONIC_EPOCH.assume_init().clone() });

pub fn time() -> u64 {
	TH_EPOCH.with(|x| {
		match u64::try_from(x.elapsed().as_nanos()) {
				Ok(v) => v,
				Err(e) => panic!("Could not convert monotonic tick to u64, err: {:?}", e)
			}
	})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_works() {
    	let t1 = time();
    	let handle = thread::spawn({move ||
    		assert!(time() > t1)
    	});
    	handle.join().unwrap();
    }
}