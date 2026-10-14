//! Enrolling a profile's key with the central store, and signing a device out.
//! A seam so a test can answer for the store; the shipped answer is
//! [`OverHttp`], which is `adapters::HttpStore`.

use std::path::Path;
use std::time::Duration;

use adapter_traits::{Secret, Store, StoreError};
use adapters::HttpStore;

pub trait Enrolling: Send + Sync {
    /// Register the key kept in `directory` with the store at `url`, as
    /// `machine`, with a device-flow `token` the store reads the account from
    /// and drops. **Answers the device id, kept beside the key** and not the
    /// token, which the caller drops.
    fn enrol(&self, url: &str, directory: &Path, token: &Secret<String>, machine: &str) -> Result<String, StoreError>;

    /// Sign `device` out of the account, signed by the key in `directory`.
    /// A profile that was never enrolled has nothing to sign out and is done.
    fn delete_device(&self, url: &str, directory: &Path, device: &str) -> Result<(), StoreError>;
}

/// The store over HTTP. **Blocking**, so called from a thread and never an
/// async worker.
pub struct OverHttp {
    timeout: Duration,
}

impl OverHttp {
    pub fn within(timeout: Duration) -> OverHttp {
        OverHttp { timeout }
    }
}

impl Enrolling for OverHttp {
    fn enrol(&self, url: &str, directory: &Path, token: &Secret<String>, machine: &str) -> Result<String, StoreError> {
        let store = HttpStore::enrol(url, directory, token, machine, self.timeout)?;
        Ok(store.device().to_string())
    }

    fn delete_device(&self, url: &str, directory: &Path, device: &str) -> Result<(), StoreError> {
        match HttpStore::open(url, directory, self.timeout).map_err(StoreError::Refused)? {
            Some(store) => store.delete_device(device),
            None => Ok(()),
        }
    }
}
