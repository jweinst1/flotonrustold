use std::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::{thread, ptr};
use crate::time_ptr::*;
use crate::configs::Configs;

// Stands for generation ptr
// this would be time instead

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

struct ThreadLocalStorage<T> {
     value:AtomicPtr<TimePtr<T>>,
     value_time:AtomicU64, // Needs it's own field because chcking on ptr is dangerous
     free_list:FreeList<T>
}

impl<T> ThreadLocalStorage<T> {
    fn new() -> ThreadLocalStorage<T> {
        ThreadLocalStorage{value:AtomicPtr::new(ptr::null_mut()), 
                           value_time:AtomicU64::new(0), 
                           free_list:FreeList::new()}
    }

    fn is_unused(&self) -> bool {
        self.value.load(Ordering::SeqCst) == ptr::null_mut()
    }
}

struct ThreadSpecificArray<T>(Vec<ThreadLocalStorage<T>>);

enum ThreadSpecificRead {
    NoUpdate,
    UpdateRecent,
    UpdateMostRecent
}

impl<T> ThreadSpecificArray<T> {
    fn new(thread_count:usize) -> ThreadSpecificArray<T> {
        let mut tlst_lst = Vec::<ThreadLocalStorage<T>>::new();
        for _ in 0..thread_count {
            tlst_lst.push(ThreadLocalStorage::new());
        }
        ThreadSpecificArray(tlst_lst)
    }

    fn time_check(&self, ctime:u64) -> bool {
        // Checks if all threads have advanced past some time, allowing safe freeing.
        for i in 0..self.0.len() {
            if self.0[i].value_time.load(Ordering::SeqCst) <= ctime {
                return false
            }
        }
        return true
    }

    fn free_run(&self, tid:u8) -> u32  {
        let flist = &self.0[tid as usize].free_list;
        if flist.1.load(Ordering::SeqCst) < Configs::get_free_list_limit() {
            return 0;
        }
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
                                    drop(Box::from_raw(rp.0));
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

    fn write(&self, tid:u8, val:*mut TimePtr<T>) {
        // need to check while still set incase this is not the owning thread
        let slot = &self.0[tid as usize];
        let loaded = slot.value.load(Ordering::SeqCst);
        if TimePtr::owned_by(loaded, tid) {
            let swapped_out = slot.value.swap(val, Ordering::SeqCst);
            slot.free_list.add(swapped_out);
        } else {
            slot.value.store(val, Ordering::SeqCst);
        }
        slot.value_time.store(TimePtr::get_time(val), Ordering::SeqCst);
    }

    fn update(&self, tid:u8) -> &ThreadLocalStorage<T> {
        let slot = &self.0[tid as usize];
        let last_read_time = slot.value_time.load(Ordering::SeqCst);

        for i in 0..self.0.len() {
            let to_check = &self.0[i];
            let to_check_time = to_check.value_time.load(Ordering::SeqCst);
            if last_read_time < to_check_time {
                // We update value first because it's safe to get a more recent value than time
                slot.value.store(to_check.value.load(Ordering::SeqCst), Ordering::SeqCst);
                slot.value_time.store(to_check_time, Ordering::SeqCst);
                break;
            }
        }
        return slot;
    }

    fn update_most_recent(&self, tid:u8) -> &ThreadLocalStorage<T> {
        let slot = &self.0[tid as usize];
        let mut last_read_time = slot.value_time.load(Ordering::SeqCst);
        let mut most_recent = 0;

        for i in 0..self.0.len() {
            let to_check_time = &self.0[i].value_time.load(Ordering::SeqCst);
            if last_read_time < *to_check_time {
                most_recent = i;
                last_read_time = *to_check_time;
            }
        }
        let to_check = &self.0[most_recent];
        let to_check_time = to_check.value_time.load(Ordering::SeqCst);
        slot.value.store(to_check.value.load(Ordering::SeqCst), Ordering::SeqCst);
        slot.value_time.store(to_check_time, Ordering::SeqCst);
        return slot;
    }

    fn read(&self, tid:u8, opt:ThreadSpecificRead) -> Option<&T> {
        let slot = match opt {
            ThreadSpecificRead::UpdateRecent => self.update(tid),
            ThreadSpecificRead::UpdateMostRecent => self.update_most_recent(tid),
            ThreadSpecificRead::NoUpdate => &self.0[tid as usize]
        };
        unsafe {
            match slot.value.load(Ordering::SeqCst).as_ref() {
                Some(r) => Some(r.0.as_ref().unwrap()),
                None => None
            }
        }
    }
}


struct ListNode<T> {
    container:ThreadSpecificArray<T>,
    next:AtomicPtr<ListNode<T>>
}

impl<T> ListNode<T> {
    fn new(thread_count:usize) -> ListNode<T> {
        ListNode{container:ThreadSpecificArray::new(thread_count), next:AtomicPtr::new(ptr::null_mut())}
    }

    fn new_ptr(thread_count:usize, next_ptr:*mut ListNode<T>) -> *mut ListNode<T> {
        Box::into_raw(Box::new(ListNode{container:ThreadSpecificArray::new(thread_count), next:AtomicPtr::new(next_ptr)}))
    }

    fn new_len(len:usize, thread_count:usize) -> *mut ListNode<T> {
        let mut base = ptr::null_mut();
        for _ in 0..len {
            base  = ListNode::new_ptr(thread_count, base);
        }
        return base;
    }

    fn get_next(ptr:*mut ListNode<T>) -> Option<*mut ListNode<T>> {
        unsafe {
            match ptr.as_ref() {
                Some(r) => Some(r.next.load(Ordering::SeqCst)),
                None => None
            }
        }
    }

    fn set_next(ptr: *mut ListNode<T>, next_ptr:*mut ListNode<T>) {
        unsafe {
            match ptr.as_ref() {
                Some(r) => r.next.store(next_ptr, Ordering::SeqCst),
                None => ()
            }
        }
    }
}

struct ListObject<T> {
    obj:AtomicPtr<ListNode<T>>,
    count:AtomicUsize
}

impl<T> ListObject<T> {
    fn new(starting_len:usize, thread_count:usize) -> ListObject<T> {
        assert!(starting_len != 0);
        ListObject{obj:AtomicPtr::new(ListNode::new_len(starting_len, thread_count)), count:AtomicUsize::new(0)}
    }

    fn extend(&self, amount:usize, thread_count:usize) {
        let mut ptr = self.obj.load(Ordering::SeqCst);
        loop {
            match ListNode::get_next(ptr) {
                Some(p) => { ptr = p; },
                None => break
            }
        }
        ListNode::set_next(ptr, ListNode::new_len(amount, thread_count));
    }

    fn write(&self, index:usize, tid:u8, val:*mut TimePtr<T>) {
        let mut ptr = self.obj.load(Ordering::SeqCst);
        for _ in 0..index {
            match ListNode::get_next(ptr) {
                Some(p) => {
                    ptr = p;
                },
                None => panic!("Out of range for write at index: {:?}", index)
            }
        }
        unsafe {
            ptr.as_ref().unwrap().container.write(tid, val);
        }
    }
}

enum ObjectForm<T> {
    Value(T),
    List(ListObject<T>)
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::sync::atomic::{AtomicPtr, AtomicI64, Ordering};

    #[test]
    fn freenode_works() {
        set_epoch();
        let tptr = TimePtr::make(30, 1);
        let fnode = FreeNode::new_ptr(tptr);
        unsafe {
            match fnode.as_ref() {
                Some(r) => assert!(r.0.load(Ordering::SeqCst) == tptr),
                None => panic!("nullptr returned from FreeNode::new_ptr")
            }
        }
        destroy_epoch();
    }

    #[test]
    fn freelist_add_works() {
        set_epoch();
        let flist = FreeList::new();
        let value:u32 = 777;
        assert!(flist.count() == 0);
        let tptr = TimePtr::make(value, 1);
        flist.add(tptr);
        unsafe {
            let checked_ptr = flist.0.load(Ordering::SeqCst).as_ref().unwrap().0.load(Ordering::SeqCst);
            assert_eq!(checked_ptr, tptr);
        }
        assert!(flist.count() == 1);
        let tptr2 = TimePtr::make(555, 1);
        flist.add(tptr2);
        unsafe {
            let checked_ptr = flist.0.load(Ordering::SeqCst).as_ref().unwrap()
                            .1.load(Ordering::SeqCst).as_ref().unwrap().0.load(Ordering::SeqCst);
            assert_eq!(checked_ptr, tptr2);
        }
        assert!(flist.count() == 2);                                                                                                                                                                                                                                                                                                                                                                                                                                                                            
        destroy_epoch();
    }

    #[test]
    fn thread_ls_works() {
        let tls = ThreadLocalStorage::<i32>::new();
        assert!(tls.value.load(Ordering::SeqCst) == ptr::null_mut());
        assert!(tls.value_time.load(Ordering::SeqCst) == 0);
    }
}
