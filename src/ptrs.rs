use std::sync::atomic::{AtomicPtr, AtomicI64, AtomicUsize, Ordering};
//use std::thread;
use std::ptr;

pub struct SharedPtr<T> {
    pub ptr:AtomicPtr<T>,
    pub counter:AtomicPtr<AtomicUsize>,
    access:AtomicI64
}

impl<T> Drop for SharedPtr<T> {
    fn drop(&mut self) {
        unsafe {
            match <*const AtomicUsize>::as_ref(self.counter.load(Ordering::SeqCst)) {
                Some(p) => if p.fetch_sub(1, Ordering::SeqCst) == 1 {
                    drop(Box::from_raw(self.ptr.load(Ordering::SeqCst)));
                    drop(Box::from_raw(self.counter.load(Ordering::SeqCst)));
                },
                None => {}
            }
        }
    }
}

impl<T> Clone for SharedPtr<T> {
    fn clone(&self) -> Self {
        if self.access.fetch_add(1, Ordering::SeqCst) < 0 {
            // This means a reset is in progress, 
            self.access.fetch_sub(1, Ordering::SeqCst);
            return SharedPtr::<T>::new(None);
        }
        let empty = SharedPtr::<T>::new(None);
        empty.ptr.store(self.ptr.load(Ordering::SeqCst), Ordering::SeqCst);
        empty.counter.store(self.counter.load(Ordering::SeqCst), Ordering::SeqCst);
        unsafe { 
            <*const AtomicUsize>::as_ref(empty.counter.load(Ordering::SeqCst)).unwrap().fetch_add(1, Ordering::SeqCst);
        }
        self.access.fetch_sub(1, Ordering::SeqCst);
        return empty;
    }
}

impl<T> SharedPtr<T> {
    pub fn new(val: Option<T>) -> SharedPtr<T> {
        match val {
            Some(v) => SharedPtr{ptr:AtomicPtr::new(Box::into_raw(Box::new(v))), 
                                counter:AtomicPtr::new(Box::into_raw(Box::new(AtomicUsize::new(1)))),
                                access:AtomicI64::new(0)},
            None => SharedPtr{ptr:AtomicPtr::new(ptr::null_mut()),
                             counter:AtomicPtr::new(ptr::null_mut()),
                             access:AtomicI64::new(0)}
        }
    }
    
    pub fn get(&self) -> *const T {
        self.ptr.load(Ordering::SeqCst)
    }

    pub fn count(&self) -> Option<usize> {
        unsafe {
            match self.counter.load(Ordering::SeqCst).as_ref() {
                Some(c) => Some(c.load(Ordering::SeqCst)),
                None => None
            }
        }
    }

    pub fn valid(&self) -> bool {
        self.ptr.load(Ordering::SeqCst) != ptr::null_mut()
    }
    
    pub fn reset(&self, val: Option<T>) -> bool {
        let current_access = self.access.load(Ordering::SeqCst);
        if current_access == 0 {
            match self.access.compare_exchange(current_access, i64::MIN, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => (),
                Err(_) => { return false; }
            }
        } else {
            return false;
        }
        match val {
            Some(v) => {
                unsafe {
                    match <*const AtomicUsize>::as_ref(self.counter.load(Ordering::SeqCst)) {
                        Some(p) => { 
                            if p.fetch_sub(1, Ordering::SeqCst) == 1 {
                               drop(Box::from_raw(self.ptr.load(Ordering::SeqCst)));
                               drop(Box::from_raw(self.counter.load(Ordering::SeqCst)));
                               println!("Dropping");
                            }
                            self.ptr.store(Box::into_raw(Box::new(v)), Ordering::SeqCst);
                            self.counter.store(Box::into_raw(Box::new(AtomicUsize::new(1))), Ordering::SeqCst);
                        },
                        None => {
                            self.ptr.store(Box::into_raw(Box::new(v)), Ordering::SeqCst);
                            self.counter.store(Box::into_raw(Box::new(AtomicUsize::new(1))), Ordering::SeqCst);
                        }
                    }
                }
            },
            None => {
                unsafe {
                    match <*const AtomicUsize>::as_ref(self.counter.load(Ordering::SeqCst)) {
                        Some(p) => {
                            if p.fetch_sub(1, Ordering::SeqCst) == 1 {
                                drop(Box::from_raw(self.ptr.load(Ordering::SeqCst)));
                                drop(Box::from_raw(self.counter.load(Ordering::SeqCst)));
                                println!("Dropping");
                            }
                            self.ptr.store(ptr::null_mut(), Ordering::SeqCst);
                            self.counter.store(ptr::null_mut(), Ordering::SeqCst);
                        },
                        None => {}
                    }
                }
            }
        }
        self.access.store(0, Ordering::SeqCst);
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_must_not_be_null() {
        let p1 = SharedPtr::new(Some(8));
        assert!(p1.get() != ptr::null_mut());
    }

    #[test]
    fn none_must_not_be_valid() {
        let p1 = SharedPtr::<i32>::new(None);
        assert!(!p1.valid()); 
    }

    #[test]
    fn drop_must_dec_count() {
        let p1 = SharedPtr::new(Some(8));
        assert!(p1.count().unwrap() == 1);
        {
            let p2 = p1.clone();
            assert!(p2.count().unwrap() == 2);
        }
        assert!(p1.count().unwrap() == 1);
    }

    #[test]
    fn st_clone_must_be_valid() {
        let p1 = SharedPtr::new(Some(8));
        let p2 = p1.clone();
        assert!(p2.valid());
        assert!(p2.count().unwrap() == 2);
    }

    #[test]
    fn reset_must_be_valid() {
        let p1 = SharedPtr::new(Some(7));
        p1.reset(Some(6));
        assert!(p1.valid());
        assert!(p1.count().unwrap() == 1);
    }

    #[test]
    fn reset_none_must_not_be_valid() {
        let p1 = SharedPtr::new(Some(7));
        p1.reset(None);
        assert!(!p1.valid());
    }

    #[test]
    fn reset_must_not_affect_clones() {
        let p1 = SharedPtr::new(Some(7));
        let p2 = p1.clone();
        p1.reset(Some(3));
        assert!(p1.valid());
        assert!(p2.valid());
        assert!(p1.count().unwrap() == 1);
        assert!(p2.count().unwrap() == 1);
        unsafe {
            assert!(*(p2.get().as_ref().unwrap()) == 7);
        }
    }

    #[test]
    fn in_progress_reset_clones_invalid() {
        let p1 = SharedPtr::new(Some(4));
        p1.access.store(i64::MIN, Ordering::SeqCst);
        let p2 = p1.clone();
        assert!(!p2.valid());
    }

    #[test]
    fn reset_blocked_if_inprog_clones() {
        let p1 = SharedPtr::new(Some(6));
        p1.access.store(1, Ordering::SeqCst);
        assert!(!p1.reset(Some(2)));
        assert!(p1.valid());
    }

    #[test]
    fn reset_blocked_if_inprog_reset() {
        let p1 = SharedPtr::new(Some(6));
        p1.access.store(-1, Ordering::SeqCst);
        assert!(!p1.reset(Some(2)));
        assert!(p1.valid());      
    }
}