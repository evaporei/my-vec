use std::alloc::{self, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, NonNull};
use std::slice;

struct RawVec<T> {
    ptr: NonNull<T>,
    cap: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        let cap = if mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };

        Self {
            ptr: NonNull::dangling(),
            cap,
        }
    }

    fn grow(&mut self) {
        assert!(mem::size_of::<T>() != 0, "capacity overflow");

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
        let elem_size = mem::size_of::<T>();

        if self.cap != 0 && elem_size != 0 {
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

struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> RawValIter<T> {
    unsafe fn new(slice: &[T]) -> Self {
        Self {
            start: slice.as_ptr(),
            end: if mem::size_of::<T>() == 0 {
                ((slice.as_ptr() as usize) + slice.len()) as *const _
            } else if slice.len() == 0 {
                slice.as_ptr()
            } else {
                slice.as_ptr().add(slice.len())
            },
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if mem::size_of::<T>() == 0 {
                    self.start = (self.start as usize + 1) as *const _;
                    Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    let old_ptr = self.start;
                    self.start = self.start.offset(1);
                    Some(ptr::read(old_ptr))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = mem::size_of::<T>();
        let len =
            (self.end as usize - self.start as usize) / if elem_size == 0 { 1 } else { elem_size };
        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for RawValIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if mem::size_of::<T>() == 0 {
                    self.end = (self.end as usize - 1) as *const _;
                    Some(ptr::read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    self.end = self.end.offset(-1);
                    Some(ptr::read(self.end))
                }
            }
        }
    }
}

pub struct MyVecIterator<T> {
    _buf: RawVec<T>, // just to own and drop
    iter: RawValIter<T>,
}

impl<T> IntoIterator for MyVec<T> {
    type Item = T;
    type IntoIter = MyVecIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        let (iter, buf) = unsafe { (RawValIter::new(&self), ptr::read(&self.buf)) };

        mem::forget(self);

        MyVecIterator { iter, _buf: buf }
    }
}

impl<T> Iterator for MyVecIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for MyVecIterator<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<T> Drop for MyVecIterator<T> {
    fn drop(&mut self) {
        for _ in &mut *self {}
    }
}

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

pub struct MyDrain<'a, T: 'a> {
    vec: PhantomData<&'a mut MyVec<T>>,
    iter: RawValIter<T>,
}

impl<'a, T> Iterator for MyDrain<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for MyDrain<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.iter.next_back()
    }
}

impl<'a, T> Drop for MyDrain<'a, T> {
    fn drop(&mut self) {
        for _ in &mut *self {}
    }
}

impl<T> MyVec<T> {
    pub fn drain(&mut self) -> MyDrain<T> {
        let iter = unsafe { RawValIter::new(&self) };

        self.len = 0;

        MyDrain {
            vec: PhantomData,
            iter,
        }
    }
}

unsafe impl<T: Send> Send for MyVec<T> {}
unsafe impl<T: Sync> Sync for MyVec<T> {}

#[test]
fn create_push_pop() {
    let mut v = MyVec::new();
    v.push(1);
    assert_eq!(1, v.len());
    assert_eq!(1, v[0]);
    for i in v.iter_mut() {
        *i += 1;
    }
    v.insert(0, 5);
    let x = v.pop();
    assert_eq!(Some(2), x);
    assert_eq!(1, v.len());
    v.push(10);
    let x = v.remove(0);
    assert_eq!(5, x);
    assert_eq!(1, v.len());
}

#[test]
fn iter_test() {
    let mut v = MyVec::new();
    for i in 0..10 {
        v.push(Box::new(i))
    }
    let mut iter = v.into_iter();
    let first = iter.next().unwrap();
    let last = iter.next_back().unwrap();
    drop(iter);
    assert_eq!(0, *first);
    assert_eq!(9, *last);
}

#[test]
fn test_drain() {
    let mut v = MyVec::new();
    for i in 0..10 {
        v.push(Box::new(i))
    }
    {
        let mut drain = v.drain();
        let first = drain.next().unwrap();
        let last = drain.next_back().unwrap();
        assert_eq!(0, *first);
        assert_eq!(9, *last);
    }
    assert_eq!(0, v.len());
    v.push(Box::new(1));
    assert_eq!(1, *v.pop().unwrap());
}

#[test]
fn test_zst() {
    let mut v = MyVec::new();
    for _i in 0..10 {
        v.push(())
    }

    let mut count = 0;

    for _ in v.into_iter() {
        count += 1
    }

    assert_eq!(10, count);
}
