use std::alloc::{self, Layout};
use std::ptr::{self, NonNull};

struct RawVec<T> {
    ptr: NonNull<T>,
    cap: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            cap: 0,
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

        self.cap = new_cap;
    }
}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            let ptr = self.ptr.as_ptr() as *mut u8;
            let layout = Layout::array::<T>(self.cap).unwrap();

            unsafe { alloc::dealloc(ptr, layout) }
        }
    }
}

pub struct MyVec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> MyVec<T> {
    pub fn new() -> Self {
        MyVec {
            buf: RawVec::new(),
            len: 0,
        }
    }

    fn grow(&mut self) {
        if self.len == self.cap() {
            self.buf.grow();
        }
    }

    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn cap(&self) -> usize {
        self.buf.cap
    }

    pub fn push(&mut self, elem: T) {
        self.grow();

        unsafe {
            ptr::write(self.ptr().add(self.len), elem);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(ptr::read(self.ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, idx: usize, elem: T) {
        assert!(idx <= self.len, "index out of bounds");

        self.grow();

        unsafe {
            ptr::copy(self.ptr().add(idx), self.ptr().add(idx + 1), self.len - idx);

            ptr::write(self.ptr().add(idx), elem);
        }

        self.len += 1;
    }

    pub fn remove(&mut self, idx: usize) -> T {
        assert!(idx < self.len, "index out of bounds");

        self.len -= 1;

        unsafe {
            let elem = ptr::read(self.ptr().add(idx));

            ptr::copy(self.ptr().add(idx + 1), self.ptr().add(idx), self.len - idx);

            elem
        }
    }
}

impl<T> Drop for MyVec<T> {
    fn drop(&mut self) {
        while let Some(_) = self.pop() {}
    }
}

use std::mem;

pub struct MyVecIterator<T> {
    _buf: RawVec<T>, // just to own and drop
    start: *const T,
    end: *const T,
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = MyVecIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        let buf = unsafe { ptr::read(&self.buf) };
        let len = self.len;
        mem::forget(self);

        MyVecIterator {
            start: buf.ptr.as_ptr(),
            end: if buf.cap == 0 {
                buf.ptr.as_ptr()
            } else {
                unsafe { buf.ptr.as_ptr().add(len) }
            },
            _buf: buf,
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
        for _ in &mut *self {}
    }
}

use std::ops::{Deref, DerefMut};
use std::slice;

impl<T> Deref for MyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr(), self.len) }
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr(), self.len) }
    }
}

unsafe impl<T: Send> Send for MyVec<T> {}
unsafe impl<T: Sync> Sync for MyVec<T> {}
