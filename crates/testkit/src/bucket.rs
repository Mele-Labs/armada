//! A [`Bucket`] that holds its objects in memory, for the tests that put a repository's runs in
//! cloud storage. It reaches nothing.
//!
//! [`FakeBucket::view`] is a second handle on the same objects with a switch of its own, so "this
//! machine cannot reach the bucket" and "another machine already wrote there" are each one
//! statement. A handle can also be set to refuse writes only, which is a key that may read.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use adapter_traits::{Bucket, BucketError};

const WORKS: u8 = 0;
const UNREACHABLE: u8 = 1;
const READ_ONLY: u8 = 2;

/// Objects in memory, by name.
#[derive(Debug, Default)]
pub struct FakeBucket {
    objects: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    mode: AtomicU8,
    calls: Mutex<Vec<String>>,
}

impl FakeBucket {
    pub fn new() -> FakeBucket {
        FakeBucket::default()
    }

    /// Another machine's handle on these objects, with its own switch and record of calls.
    pub fn view(&self) -> FakeBucket {
        FakeBucket { objects: Arc::clone(&self.objects), ..FakeBucket::default() }
    }

    /// While set, every call from this handle fails as unreachable and changes nothing.
    pub fn set_unreachable(&self, unreachable: bool) {
        self.mode.store(if unreachable { UNREACHABLE } else { WORKS }, Ordering::SeqCst);
    }

    /// While set, reads answer and every write is refused.
    pub fn set_read_only(&self, read_only: bool) {
        self.mode.store(if read_only { READ_ONLY } else { WORKS }, Ordering::SeqCst);
    }

    /// An object, whichever handle wrote it.
    pub fn object(&self, name: &str) -> Option<Vec<u8>> {
        self.objects.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(name).cloned()
    }

    /// The names of every object held.
    pub fn names(&self) -> Vec<String> {
        self.objects.lock().unwrap_or_else(std::sync::PoisonError::into_inner).keys().cloned().collect()
    }

    /// Put an object there as another tool would have.
    pub fn plant(&self, name: &str, body: &[u8]) {
        self.objects.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(name.to_string(), body.to_vec());
    }

    /// The calls this handle took, `get name` or `put name`, **including those that failed**.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    fn took(&self, call: String) {
        self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(call);
    }
}

impl Bucket for FakeBucket {
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, BucketError> {
        self.took(format!("get {name}"));
        if self.mode.load(Ordering::SeqCst) == UNREACHABLE {
            return Err(BucketError::Unreachable("the fake bucket is switched off".to_string()));
        }
        Ok(self.object(name))
    }

    fn put(&self, name: &str, body: &[u8]) -> Result<(), BucketError> {
        self.took(format!("put {name}"));
        match self.mode.load(Ordering::SeqCst) {
            UNREACHABLE => Err(BucketError::Unreachable("the fake bucket is switched off".to_string())),
            READ_ONLY => Err(BucketError::Refused("status 403: the key may not write here".to_string())),
            _ => {
                self.plant(name, body);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_reads_what_another_wrote_and_a_switched_off_one_changes_nothing() {
        let a = FakeBucket::new();
        let b = a.view();
        a.put("runs/shop.toml", b"one").expect("written");
        assert_eq!(b.get("runs/shop.toml").expect("read"), Some(b"one".to_vec()));
        b.set_unreachable(true);
        assert!(b.put("runs/shop.toml", b"two").expect_err("down").is_unreachable());
        assert_eq!(a.object("runs/shop.toml"), Some(b"one".to_vec()));
        b.set_read_only(true);
        assert!(b.get("runs/shop.toml").is_ok());
        assert!(!b.put("runs/shop.toml", b"two").expect_err("refused").is_unreachable());
    }
}
