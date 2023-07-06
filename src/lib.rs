use std::alloc::{self, Layout};
use std::ptr::{self, NonNull};

pub struct MyVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    len: usize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        MyVec {
            ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = 2 * self.cap;

            let new_layout = Layout::array::<T>(new_cap).unwrap();
            (new_cap, new_layout)
        };

        assert!(
            new_layout.size() <= isize::MAX as usize,
            "allocation too large"
        );

        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        self.ptr = NonNull::new(new_ptr as *mut T)
            .unwrap_or_else(|| alloc::handle_alloc_error(new_layout));

        // self.ptr = match NonNull::new(new_ptr as *mut T) {
        //     Some(p) => p,
        //     None => alloc::handle_alloc_error(new_layout),
        // };

        // self.ptr = if !new_ptr.is_null() {
        //     NonNull::new(new_ptr as *mut T).unwrap()
        // } else {
        //     alloc::handle_alloc_error(new_layout)
        // };

        self.cap = new_cap;
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap {
            self.grow()
        }

        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len), elem);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr.as_ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, idx: usize, elem: T) {
        assert!(idx <= self.len, "index out of bounds");

        if self.len == self.cap {
            self.grow()
        }

        unsafe {
            ptr::copy(
                self.ptr.as_ptr().add(idx),
                self.ptr.as_ptr().add(idx + 1),
                self.len - idx,
            );

            ptr::write(self.ptr.as_ptr().add(idx), elem);
        }

        self.len += 1;
    }

    pub fn remove(&mut self, idx: usize) -> T {
        assert!(idx < self.len, "index out of bounds");

        self.len -= 1;

        unsafe {
            let elem = ptr::read(self.ptr.as_ptr().add(idx));

            ptr::copy(
                self.ptr.as_ptr().add(idx + 1),
                self.ptr.as_ptr().add(idx),
                self.len - idx,
            );

            elem
        }
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            while let Some(_) = self.pop() {}

            let ptr = self.ptr.as_ptr() as *mut u8;
            let layout = Layout::array::<T>(self.cap).unwrap();

            unsafe { alloc::dealloc(ptr, layout) }
        }
    }
}

use std::mem::{self, ManuallyDrop};

pub struct MyVecIterator<T> {
    buf: NonNull<T>,
    cap: usize,
    start: *const T,
    end: *const T,
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = MyVecIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        let vec = ManuallyDrop::new(self);

        let ptr = vec.ptr;
        let cap = vec.cap;
        let len = vec.len;

        MyVecIterator {
            buf: ptr,
            cap,
            start: ptr.as_ptr(),
            end: if cap == 0 {
                ptr.as_ptr()
            } else {
                unsafe { ptr.as_ptr().add(len) }
            },
        }
    }
}

impl<T> Iterator for MyVecIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let elem = ptr::read(self.start);
                self.start = self.start.offset(1);
                Some(elem)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = (self.end as usize - self.start as usize) / mem::size_of::<T>();
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for MyVecIterator<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.end = self.end.offset(-1);
                Some(ptr::read(self.end))
            }
        }
    }
}

impl<T> Drop for MyVecIterator<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            for _ in &mut *self {}
            let ptr = self.buf.as_ptr() as *mut u8;
            let layout = Layout::array::<T>(self.cap).unwrap();
            unsafe { alloc::dealloc(ptr, layout) }
        }
    }
}

use std::ops::{Deref, DerefMut};
use std::slice;

impl<T> Deref for MyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

unsafe impl<T: Send> Send for MyVec<T> {}
unsafe impl<T: Sync> Sync for MyVec<T> {}
