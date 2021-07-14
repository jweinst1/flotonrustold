use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicU32, AtomicU64, Ordering};
use std::{thread, ptr};
use std::time::{Duration, Instant};
use std::convert::TryFrom;

static MONOTONIC_EPOCH:AtomicPtr<Instant> = AtomicPtr::new(ptr::null_mut());
static FREE_LIST_DEFAULT:u32 = 10;
static FREE_LIST_LIM:AtomicU32 = AtomicU32::new(FREE_LIST_DEFAULT);
static THREAD_COUNT:AtomicUsize = AtomicUsize::new(8);

pub fn get_thread_count() -> usize {
    THREAD_COUNT.load(Ordering::SeqCst)
}

pub fn set_thread_count(count:usize) {
    THREAD_COUNT.store(count, Ordering::SeqCst);
}

pub fn set_free_list_lim(limit:u32) {
    FREE_LIST_LIM.store(limit, Ordering::SeqCst);
}

// only call once
pub fn set_epoch() {
	let swapped_out = MONOTONIC_EPOCH.swap(Box::into_raw(Box::new(Instant::now())), Ordering::SeqCst);
    if swapped_out != ptr::null_mut() {
        unsafe { drop(Box::from_raw(swapped_out)); }
    }
}

pub fn check_time() -> u64 {
	unsafe {
		match MONOTONIC_EPOCH.load(Ordering::SeqCst).as_ref() {
			// todo, configure precision
			Some(r) => match u64::try_from(r.elapsed().as_nanos()) {
				Ok(v) => v,
				Err(e) => panic!("Could not convert monotonic tick to u64, err: {:?}", e)
			},
			None => panic!("MONOTONIC_EPOCH was loaded but not initialized!")
		}
	}
}

pub struct TimePtr<T>(pub T, pub u64);

impl<T> TimePtr<T> {
    pub fn make(val:T) -> *mut TimePtr<T> {
        Box::into_raw(Box::new(TimePtr(val, check_time())))
    }
    
    pub fn get_time(ptr:*mut TimePtr<T>) -> Option<u64> {
        unsafe {
            match ptr.as_ref() {
                Some(r) => Some(r.1),
                None => None
            }
        }
    }
}

// This is not actually thread safe, this should only be called by a specific thread
// but we need this to trick rust's strict mutable borrow checker.
struct FreeNode<T>(AtomicPtr<TimePtr<T>>, AtomicPtr<FreeNode<T>>);

impl<T> FreeNode<T> {
    fn new() -> *mut FreeNode<T> {
        Box::into_raw(Box::new(FreeNode(AtomicPtr::new(ptr::null_mut()), AtomicPtr::new(ptr::null_mut()))))
    }

    fn new_ptr(ptr:*mut TimePtr<T>) -> *mut FreeNode<T> {
        Box::into_raw(Box::new(FreeNode(AtomicPtr::new(ptr), AtomicPtr::new(ptr::null_mut()))))
    }
}
// This is not actually thread safe, this should only be called by a specific thread
// but we need this to trick rust's strict mutable borrow checker.
struct FreeList<T>(AtomicPtr<FreeNode<T>>, AtomicU32);

impl<T> FreeList<T> {
    fn new() -> FreeList<T> {
        FreeList(AtomicPtr::new(FreeNode::new()), AtomicU32::new(0))
    }

    fn count(&self) -> u32 {
        self.1.load(Ordering::SeqCst)
    }

    fn add(&self, ptr:*mut TimePtr<T>) {
        let mut list_ptr  = self.0.load(Ordering::SeqCst);
        loop{
            unsafe {
                match list_ptr.as_ref() {
                    Some(r) => {
                        if r.0.load(Ordering::SeqCst) == ptr::null_mut() {
                            r.0.store(ptr, Ordering::SeqCst);
                            break;
                        }
                        let next_ptr = r.1.load(Ordering::SeqCst);
                        if next_ptr == ptr::null_mut() {
                            r.1.store(FreeNode::new_ptr(ptr), Ordering::SeqCst);
                            break;
                        } else {
                            list_ptr = next_ptr;
                        }
                    },
                    None => panic!("Overran the free list in add()")
                }
            }
        }
        self.1.fetch_add(1, Ordering::SeqCst);
    }
}

struct ThreadStorage<T> {
    cur_time:AtomicU64,
    free_list:FreeList<T>
}

pub struct Shared<T> {
    time_keeps:Vec<ThreadStorage<T>>,
    cur_ptr:AtomicPtr<TimePtr<T>>
}

impl<T> Shared<T> {
    pub fn new() -> Shared<T> {
        let mut ts_vec = vec![];
        let tc = get_thread_count();
        for _ in 0..tc {
            ts_vec.push(ThreadStorage{cur_time:AtomicU64::new(0), free_list:FreeList::new()});
        }
        Shared{time_keeps:ts_vec, cur_ptr:AtomicPtr::new(ptr::null_mut())}
    }
    
    pub fn t_count(&self) -> usize {
        self.time_keeps.len()
    }
    
    pub fn time_check(&self, ctime:u64) -> bool {
        // Checks if all threads have advanced past some time, allowing safe freeing.
        for i in 0..self.time_keeps.len() {
            if self.time_keeps[i].cur_time.load(Ordering::SeqCst) < ctime {
                return false
            }
        }
        return true
    }

    pub fn free_run(&self, tid:usize) -> u32  {
        let flist = &self.time_keeps[tid].free_list;
        if flist.1.load(Ordering::SeqCst) < FREE_LIST_LIM.load(Ordering::SeqCst) {
            return 0;
        }
        //println!("Running free list on thread: {}", tid);
        let mut freed = 0;
        let mut cur_ptr = flist.0.load(Ordering::SeqCst);
        loop {
            unsafe {
                match cur_ptr.as_ref() {
                    Some(r) => {
                        let inner_ptr = r.0.load(Ordering::SeqCst);
                        match inner_ptr.as_ref()  {
                            Some(rp) => {
                                if self.time_check(rp.1) {
                                    drop(Box::from_raw(inner_ptr));
                                    freed += 1;
                                    r.0.store(ptr::null_mut(), Ordering::SeqCst);
                                }
                            },
                            None => ()
                        }
                        cur_ptr = r.1.load(Ordering::SeqCst); 
                    },
                    None => { break; }
                }
            }
        }
        flist.1.fetch_sub(freed, Ordering::SeqCst);
        return freed;
    }
    
    pub fn write(&self, ptr:*mut TimePtr<T>, tid:usize) {
        let swapped_out = self.cur_ptr.swap(ptr, Ordering::SeqCst);
        match TimePtr::get_time(swapped_out) {
            Some(ti) => {
                let time_slot = & self.time_keeps[tid];
                time_slot.cur_time.store(ti, Ordering::SeqCst);
                time_slot.free_list.add(swapped_out);
            },
            None => ()
        }
    }
    
    pub fn read(&self, tid:usize) -> *mut TimePtr<T> {
        self.free_run(tid);
        let read_ptr = self.cur_ptr.load(Ordering::SeqCst);
        let time_slot = & self.time_keeps[tid];
        match TimePtr::get_time(read_ptr) {
            Some(ti) => {
                time_slot.cur_time.store(ti, Ordering::SeqCst);
                read_ptr
            },
            None => ptr::null_mut()
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn epoch_timer_works() {
        set_epoch();
        let t1 = check_time();
        let t2 = check_time();
        if !(t1 < t2) {
            panic!("monotonic t1:{:?} is not less than t2:{:?}", t1, t2);
        }
    }

    #[test]
    fn freenode_works() {
        set_epoch();
        let tptr = TimePtr::make(30);
        let fnode = FreeNode::new_ptr(tptr);
        unsafe {
            match fnode.as_ref() {
                Some(r) => assert!(r.0.load(Ordering::SeqCst) == tptr),
                None => panic!("nullptr returned from FreeNode::new_ptr")
            }
        }
    }

    #[test]
    fn freelist_add_works() {
        set_epoch();
        let flist = FreeList::new();
        let value:u32 = 777;
        assert!(flist.count() == 0);
        let tptr = TimePtr::make(value);
        flist.add(tptr);
        unsafe {
            let checked_ptr = flist.0.load(Ordering::SeqCst).as_ref().unwrap().0.load(Ordering::SeqCst);
            assert_eq!(checked_ptr, tptr);
        }
        assert!(flist.count() == 1);
        let tptr2 = TimePtr::make(555);
        flist.add(tptr2);
        unsafe {
            let checked_ptr = flist.0.load(Ordering::SeqCst).as_ref().unwrap()
                            .1.load(Ordering::SeqCst).as_ref().unwrap().0.load(Ordering::SeqCst);
            assert_eq!(checked_ptr, tptr2);
        }
        assert!(flist.count() == 2);
    }

    #[test]
    fn shared_init_works() {
        let tcdef = THREAD_COUNT.load(Ordering::SeqCst);
        let shared = Shared::<TestType>::new();
        assert!(shared.t_count() == tcdef);
        assert!(!shared.time_check(1));
    }

    #[test]
    fn shared_freerun_works() {
        // We want control of free list just for this test
        set_free_list_lim(50);
        assert!(THREAD_COUNT.load(Ordering::SeqCst) > 1);
        set_epoch();
        let shared = Shared::<TestType>::new();
        shared.write(TimePtr::make(TestType(5)), 0);
        assert!(shared.free_run(0) == 0);
        shared.write(TimePtr::make(TestType(5)), 0);
        assert!(shared.free_run(0) == 0);
        set_free_list_lim(1);
        // still shouldn't free since other thread slots 
        assert!(shared.free_run(0) == 0);
        set_free_list_lim(FREE_LIST_DEFAULT);
    }
}
