//! How many agents Fleet may have working at one time.
//!
//! **The bound is configured, not computed.** [`Concurrency`] is handed in by
//! the composition root as the shipped number, replaced by a saved one at
//! assembly and again whenever a person saves — `crate::limits` — and
//! `settings.toml`'s `concurrency-cap` row is the specification it satisfies.
//! Machine headroom is `crate::headroom`'s, a separate question. What it bounds
//! now is the width of a build: `checks_runner::CheckWidth` is read off it.

/// How many agents Fleet may be working at one time.
///
/// **No `Default`**: the number is a decision somebody made and wrote down, and a
/// type that supplies one lets a caller not make it.
///
/// Zero is not expressible. [`Concurrency::of`] raises a zero to one rather
/// than admitting a value with nothing under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Concurrency(usize);

impl Concurrency {
    pub const fn of(jobs: usize) -> Concurrency {
        Concurrency(if jobs == 0 { 1 } else { jobs })
    }

    pub const fn jobs(&self) -> usize {
        self.0
    }
}
