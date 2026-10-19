//! A bucket: named blobs a person owns in a cloud provider's storage, where a repository's runs
//! can be kept when the central store is not the place they want them.
//!
//! **Two calls and no more.** Fleet reads one object and writes one object; it lists nothing and
//! deletes nothing, so a bucket it is handed can be written with credentials that may do no more.
//!
//! # Unreachable and refused are different errors
//!
//! As [`StoreError`](crate::StoreError): a write that could not be confirmed is kept by the caller
//! and tried again, and a refusal says what to change.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Why the bucket did not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BucketError {
    /// Nothing answered: no network, a timeout, a server that is down.
    Unreachable(String),
    /// Something answered and said no: a key the provider rejected, a bucket that is not there, an
    /// answer this code cannot read.
    Refused(String),
    /// This machine has nothing to sign with. Nothing was sent.
    Unsigned(String),
}

impl BucketError {
    pub fn is_unreachable(&self) -> bool {
        matches!(self, BucketError::Unreachable(_))
    }

    /// A sentence for a person.
    pub fn said(&self) -> String {
        match self {
            BucketError::Unreachable(why) => {
                let mut out = "The bucket could not be reached: ".to_string();
                out.push_str(why);
                out
            }
            BucketError::Refused(why) => {
                let mut out = "The bucket refused the request: ".to_string();
                out.push_str(why);
                out
            }
            BucketError::Unsigned(why) => why.clone(),
        }
    }
}

/// One bucket and the prefix under it, as Fleet reads and writes it. **Every call is a network
/// round trip**, so a caller holds it on a blocking thread.
pub trait Bucket {
    /// The object called `name` under the prefix, or `None` where there is none.
    fn get(&self, name: &str) -> Result<Option<Vec<u8>>, BucketError>;

    /// Write `body` as the object called `name` under the prefix, replacing one that is there.
    fn put(&self, name: &str, body: &[u8]) -> Result<(), BucketError>;
}
