use std::cmp::Ordering::*;
use std::mem;
use std::ptr;
use std::sync::{Mutex, MutexGuard};

use crate::ConcurrentSet;

#[derive(Debug)]
struct Node<T> {
    data: T,
    next: Mutex<*mut Node<T>>,
}

/// Concurrent sorted singly linked list using fine-grained lock-coupling.
#[derive(Debug)]
pub struct FineGrainedListSet<T> {
    head: Mutex<*mut Node<T>>,
}

unsafe impl<T: Send> Send for FineGrainedListSet<T> {}
unsafe impl<T: Send> Sync for FineGrainedListSet<T> {}

/// Reference to the `next` field of previous node which points to the current node.
///
/// For example, given the following linked list:
///
/// ```text
/// head -> 1 -> 2 -> 3 -> null
/// ```
///
/// If `cursor` is currently at node 2, then `cursor.0` should be the `MutexGuard` obtained from the
/// `next` of node 1. In particular, `cursor.0.as_ref().unwrap()` creates a shared reference to node
/// 2.
struct Cursor<'l, T>(MutexGuard<'l, *mut Node<T>>);

impl<T> Node<T> {
    fn new(data: T, next: *mut Self) -> *mut Self {
        Box::into_raw(Box::new(Self {
            data,
            next: Mutex::new(next),
        }))
    }
}

impl<T: Ord> Cursor<'_, T> {
    /// Moves the cursor to the position of key in the sorted list.
    /// Returns whether the value was found.
    fn find(&mut self, key: &T) -> bool {
        unsafe {
            if let Some(node) = self.0.as_ref() {
                if &node.data == key {
                    return true;
                }
                if &node.data > key {
                    return false;
                }
                let mut nxt = node.next.lock().unwrap();
                *self = Cursor(nxt);
                self.find(key)
            } else {
                false
            }
        }
    }
}

impl<T> FineGrainedListSet<T> {
    /// Creates a new list.
    pub fn new() -> Self {
        Self {
            head: Mutex::new(ptr::null_mut()),
        }
    }
}

impl<T: Ord> FineGrainedListSet<T> {
    fn find(&self, key: &T) -> (bool, Cursor<'_, T>) {
        let mut cursor = Cursor(self.head.lock().unwrap());
        let found = cursor.find(key);
        (found, cursor)
    }
}

impl<T: Ord> ConcurrentSet<T> for FineGrainedListSet<T> {
    fn contains(&self, key: &T) -> bool {
        self.find(key).0
    }

    fn insert(&self, key: T) -> bool {
        let (found, mut cursor) = self.find(&key);
        if found {
            return false;
        }
        let mut lock = cursor.0;
        let next = *lock;
        let new_node = Node::new(key, next);
        *lock = new_node;
        true
    }

    fn remove(&self, key: &T) -> bool {
        let (found, mut cursor) = self.find(key);
        if !found {
            return false;
        }
        let mut lock = cursor.0;
        // unsafe {
        //     if let Some(node) = lock.as_mut() {
        //         let to_remove = ptr::replace(&mut node.next, ptr::null_mut());

        //         let next = node.next.lock().unwrap();
        //         *lock = *next;
        //     } else {
        //         *lock = ptr::null_mut();
        //     }
        // }
        unsafe {
            if let Some(node) = (lock).as_mut() {
                let to_remove = ptr::replace(&mut *node.next.lock().unwrap(), ptr::null_mut());
                let next_next = (*to_remove).next.lock().unwrap();
                let _ = mem::replace(&mut *lock, *next_next);
                // *lock = *next_next;
                // ptr::write(lock, *next_next);
                // Convert the raw pointer back into a Box to deallocate it
                let _ = Box::from_raw(to_remove);
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct Iter<'l, T> {
    cursor: MutexGuard<'l, *mut Node<T>>,
}

impl<T> FineGrainedListSet<T> {
    /// An iterator visiting all elements.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            cursor: self.head.lock().unwrap(),
        }
    }
}

impl<'l, T> Iterator for Iter<'l, T> {
    type Item = &'l T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if let Some(node) = self.cursor.as_ref() {
                self.cursor = node.next.lock().unwrap();
                Some(&node.data)
            } else {
                None
            }
        }
    }
}

impl<T> Drop for FineGrainedListSet<T> {
    fn drop(&mut self) {
        for _ in self.iter() {}
    }
}

impl<T> Default for FineGrainedListSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
