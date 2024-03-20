//! Thread-safe key/value cache.

use std::collections::hash_map::{Entry, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

/// Cache that remembers the result for each key.
#[derive(Debug, Default)]
pub struct Cache<K, V> {
    // todo! This is an example cache type. Build your own cache type that satisfies the
    // specification for `get_or_insert_with`.
    // inner: Mutex<HashMap<K, V>>,
    inner: Arc<RwLock<HashMap<K, Arc<RwLock<Option<V>>>>>>,
}

// impl<K, V> Default for Cache<K, V> {
//     fn default() -> Self {
//         Self {
//             inner: RwLock::new(HashMap::new()),
//         }
//     }
// }

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// Retrieve the value or insert a new one created by `f`.
    ///
    /// An invocation to this function should not block another invocation with a different key. For
    /// example, if a thread calls `get_or_insert_with(key1, f1)` and another thread calls
    /// `get_or_insert_with(key2, f2)` (`key1≠key2`, `key1,key2∉cache`) concurrently, `f1` and `f2`
    /// should run concurrently.
    ///
    /// On the other hand, since `f` may consume a lot of resource (= money), it's undesirable to
    /// duplicate the work. That is, `f` should be run only once for each key. Specifically, even
    /// for concurrent invocations of `get_or_insert_with(key, f)`, `f` is called only once per key.
    ///
    /// Hint: the [`Entry`] API may be useful in implementing this function.
    ///
    /// [`Entry`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.entry
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        // let value = Arc::new(RwLock::new(None));
        // let mut write_lock = self.inner.write().unwrap();

        // if !write_lock.contains_key(&key) {
        //     write_lock.insert(key.clone(), Arc::clone(&value));
        // }

        // let stored_value = write_lock.get(&key).unwrap().clone();
        // drop(write_lock);

        // {
        //     let r = stored_value.read().unwrap();
        //     if r.is_none() {
        //         drop(r);
        //         let v = f(key);
        //         let mut w = stored_value.write().unwrap();
        //         *w = Some(v.clone());
        //         return v.clone();
        //     } else {
        //         r.clone().unwrap()
        //     }

        //     // stored_value.read().unwrap().unwrap()
        // }
        let mut write_lock = self.inner.write().unwrap();
        match write_lock.entry(key.clone()) {
            Entry::Occupied(entry) => {
                let stored_value = entry.get().clone();
                let read_guard = stored_value.read().unwrap();
                read_guard.clone().unwrap()
            }
            Entry::Vacant(entry) => {
                let value = Arc::new(RwLock::new(None));
                entry.insert(value.clone());
                drop(write_lock);
                let returned_value = f(key);
                let mut write_guard = value.write().unwrap();
                *write_guard = Some(returned_value.clone());
                returned_value
            }
        }
    }
}
