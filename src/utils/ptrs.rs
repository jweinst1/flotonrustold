use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, Ordering};
use std::thread;
use std::ptr;

pub struct SharedPtr<T> {
    pub ptr:AtomicPtr<T>,
    pub counter:AtomicPtr<AtomicUsize>
}

impl<T> Drop for SharedPtr<T> {
    fn drop(&mut self) {
        unsafe {
            match <*const AtomicUsize>::as_ref(self.counter.load(Ordering::SeqCst)) {
                Some(p) => if p.fetch_sub(1, Ordering::SeqCst) == 1 {
                    drop(Box::from_raw(self.ptr.load(Ordering::SeqCst)));
                    drop(Box::from_raw(self.counter.load(Ordering::SeqCst)));
                    println!("Dropping");
                },
                None => {}
            }
        }
    }
}

impl<T> Clone for SharedPtr<T> {
    fn clone(&self) -> Self {
        let empty = SharedPtr::<T>::new(None);
        empty.ptr.store(self.ptr.load(Ordering::SeqCst), Ordering::SeqCst);
        empty.counter.store(self.counter.load(Ordering::SeqCst), Ordering::SeqCst);
        unsafe { 
            <*const AtomicUsize>::as_ref(empty.counter.load(Ordering::SeqCst)).unwrap().fetch_add(1, Ordering::SeqCst);
        }
        return empty;
    }
}

impl<T> SharedPtr<T> {
    pub fn new(val: Option<T>) -> SharedPtr<T> {
        match val {
            Some(v) => SharedPtr{ptr:AtomicPtr::new(Box::into_raw(Box::new(v))), 
                                counter:AtomicPtr::new(Box::into_raw(Box::new(AtomicUsize::new(1))))},
            None => SharedPtr{ptr:AtomicPtr::new(ptr::null_mut()),
                             counter:AtomicPtr::new(ptr::null_mut())}
        }
    }
    
    pub fn get(&self) -> *const T {
        self.ptr.load(Ordering::SeqCst)
    }
    
    pub fn reset(&self, val: Option<T>) {
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
    }
}