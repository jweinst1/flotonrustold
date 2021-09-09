use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::{thread, ptr};
use std::time::{Duration, Instant};
use crate::tlocal;
use crate::traits::NewType;
use crate::trie::IntTrie;

#[derive(Debug)]
pub struct TimePtr<T>(pub T, pub u64);

impl<T> TimePtr<T> {
    pub fn make(val:T) -> *mut TimePtr<T> {
        alloc!(TimePtr(val, tlocal::time()))
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
#[derive(Debug)]
struct FreeNode<T>(AtomicPtr<TimePtr<T>>, AtomicPtr<FreeNode<T>>);

impl<T> FreeNode<T> {
    fn new() -> *mut FreeNode<T> {
        alloc!(FreeNode(AtomicPtr::new(ptr::null_mut()), AtomicPtr::new(ptr::null_mut())))
    }

    fn new_ptr(ptr:*mut TimePtr<T>) -> *mut FreeNode<T> {
        alloc!(FreeNode(AtomicPtr::new(ptr), AtomicPtr::new(ptr::null_mut())))
    }
}
// This is not actually thread safe, this should only be called by a specific thread
// but we need this to trick rust's strict mutable borrow checker.
#[derive(Debug)]
struct FreeList<T>(AtomicPtr<FreeNode<T>>, AtomicU32);

impl<T> Drop for FreeList<T> {
    fn drop(&mut self) {
        let mut cur_ptr = self.0.load(Ordering::Relaxed);
        while let Some(noderef) = unsafe {cur_ptr.as_ref()} {
            let old_ptr = cur_ptr;
            let old_time_ptr = noderef.0.load(Ordering::Relaxed);
            if nonull!(old_time_ptr) {
                free!(old_time_ptr);
            }
            cur_ptr = noderef.1.load(Ordering::Relaxed);
            free!(old_ptr);
        }
    }
}

impl<T> NewType for FreeList<T> {
    fn new() -> Self {
        FreeList(AtomicPtr::new(FreeNode::new()), AtomicU32::new(0))
    }
}

impl<T> FreeList<T> {

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

#[derive(Debug)]
struct ThreadStorage<T> {
    cur_time:AtomicU64,
    free_list:FreeList<T>
}

impl<T> NewType for ThreadStorage<T> {
    fn new() -> Self {
        ThreadStorage{cur_time:AtomicU64::new(0), free_list:FreeList::new()}
    }
}

#[derive(Debug)]
pub struct Shared<T> {
    time_keeps:IntTrie<ThreadStorage<T>>,
    cur_ptr:AtomicPtr<TimePtr<T>>
}

impl<T> Drop for Shared<T> {
    fn drop(&mut self) {
        let current = self.cur_ptr.load(Ordering::SeqCst);
        if nonull!(current) {
            free!(current);
        }
    }
}

impl<T> NewType for Shared<T> {
    fn new() -> Shared<T> {
        // todo make configurable
        Shared{time_keeps:IntTrie::new(5), cur_ptr:newptr!()}
    }
}

impl<T> Shared<T> {

    pub fn new_val(val:T) -> Shared<T> {
        let made = Shared::new();
        made.cur_ptr.store(TimePtr::make(val), Ordering::SeqCst);
        made
    }
    
    pub fn time_check(&self, ctime:u64) -> bool {
        // Checks if all threads have advanced past some time, allowing safe freeing.

        fn check_time_keep<T>(stor:&ThreadStorage<T>, op:&u64) -> bool {
            stor.cur_time.load(Ordering::SeqCst) < *op
        }
        return self.time_keeps.check_if_one(check_time_keep, &ctime);
    }

    pub fn free_run(&self) -> u32  {
        let flist = &self.time_keeps.get_by_tid().free_list;
        if flist.count() < tlocal::free_lim() {
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
                                    free!(inner_ptr);
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
    
    pub fn write(&self, ptr:*mut TimePtr<T>) {
        let swapped_out = self.cur_ptr.swap(ptr, Ordering::SeqCst);
        match TimePtr::get_time(swapped_out) {
            Some(ti) => {
                let time_slot = & self.time_keeps.get_by_tid();
                time_slot.cur_time.store(ti, Ordering::SeqCst);
                time_slot.free_list.add(swapped_out);
            },
            None => ()
        }
    }
    
    pub fn read(&self) -> *mut TimePtr<T> {
        self.free_run();
        let read_ptr = self.cur_ptr.load(Ordering::SeqCst);
        let time_slot = & self.time_keeps.get_by_tid();
        match TimePtr::get_time(read_ptr) {
            Some(ti) => {
                time_slot.cur_time.store(ti, Ordering::SeqCst);
                read_ptr
            },
            None => ptr::null_mut()
        }
    }

    pub fn update_time(&self) -> bool {
        match TimePtr::get_time(self.cur_ptr.load(Ordering::SeqCst)) {
            Some(ti) => {
                self.time_keeps.get_by_tid().cur_time.store(ti, Ordering::SeqCst);
                true
            },
            None => false
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::threading::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};
    #[derive(Debug, Copy, Clone)]
    struct TestType(u32);

    #[test]
    fn freenode_works() {
        tlocal::set_epoch();
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
        tlocal::set_epoch();
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
    fn shared_freerun_works() {
        // We want control of free list just for this test
        tlocal::set_free_lim(50);
        tlocal::set_epoch();
        let mut shared = Shared::<TestType>::new();
        // init test
        assert!(!shared.time_check(0));
        let t1 = thcall!(5, shared.write(TimePtr::make(TestType(5))));
        assert!(shared.free_run() == 0);
        let t2 = thcall!(5, shared.write(TimePtr::make(TestType(5))));
        assert!(shared.free_run() == 0);
        tlocal::set_free_lim(1);
        // still shouldn't free since other thread slots 
        assert!(shared.free_run() == 0);
        tlocal::set_free_lim(3);
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn shared_timecheck_works() {
        tlocal::set_epoch();
        let mut shared = Shared::<TestType>::new();
        let to_write = TimePtr::make(TestType(5));
        shared.write(to_write);
        let t1 = thcall!(5, shared.write(TimePtr::make(TestType(5))));
        let t2 = thcall!(5, shared.write(TimePtr::make(TestType(5))));
        assert!(!shared.time_check(TimePtr::get_time(to_write).unwrap()));
        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn shared_rw_works() {
        tlocal::set_epoch();
        let shared = Shared::<TestType>::new();
        let to_write = TimePtr::make(TestType(5));
        shared.write(to_write);
        assert_eq!(to_write, shared.cur_ptr.load(Ordering::SeqCst));
        assert_eq!(to_write, shared.read());
    }

    #[test]
    fn shared_rw_time_works() {
        tlocal::set_epoch();
        let shared = Shared::<TestType>::new();
        let to_write1 = TimePtr::make(TestType(5));
        let to_write2 = TimePtr::make(TestType(1));
        let wtime1 = TimePtr::get_time(to_write1).unwrap();
        let wtime2 = TimePtr::get_time(to_write2).unwrap();
        shared.write(to_write1);
        shared.read();
        let seen_time1 = shared.time_keeps.get_by_tid().cur_time.load(Ordering::SeqCst);
        shared.write(to_write2);
        shared.read();
        let seen_time2 = shared.time_keeps.get_by_tid().cur_time.load(Ordering::SeqCst);
        assert_eq!(seen_time1, wtime1);
        assert_eq!(seen_time2, wtime2);
        assert!(seen_time1 < seen_time2);
    }

    #[test]
    fn shared_update_time_works() {
        tlocal::set_epoch();
        let shared = Shared::<TestType>::new();
        shared.write(TimePtr::make(TestType(5)));
        let seen_time1 = shared.time_keeps.get_by_tid().cur_time.load(Ordering::SeqCst);
        assert!(shared.update_time());
        let seen_time2 = shared.time_keeps.get_by_tid().cur_time.load(Ordering::SeqCst);
        assert!(seen_time1 < seen_time2);
    }
}
