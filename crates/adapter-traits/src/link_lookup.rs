//! What a bare link inside a request might resolve to, rendered as a process
//! and never run here.
//!
//! # The same split as a Judge call, for the same reason
//!
//! [`LinkLookup::resolve`] answers with a [`LookupCall`] — a program and its
//! arguments — and does not run it. Fleet's own process is the one with a
//! network; this crate stays free of one, which is what lets `resolve` be
//! total rather than asynchronous. Rendering cannot fail: a request naming
//! nothing this can resolve simply gets `None`, not an error.
//!
//! # What a [`LookupCall`] cannot say
//!
//! No directory and no stdin, unlike [`crate::JudgeCall`]. What comes back on
//! this call's stdout is the whole answer, and there is nothing else to give
//! it or point it at.

use alloc::string::String;
use alloc::vec::Vec;

/// A process that would resolve one link, if it were run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupCall {
    program: String,
    args: Vec<String>,
}

impl LookupCall {
    /// The only way to make one — a program and the arguments to run it with.
    pub fn rendered(program: &str, args: Vec<String>) -> LookupCall {
        LookupCall {
            program: String::from(program),
            args,
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }
}

/// Whether a request names something worth fetching before a Drone ever sees
/// it, and what to run if it does.
///
/// **`None` is the ordinary answer.** Most requests name nothing this can
/// resolve, and a caller checking that first is cheaper than one that always
/// starts a process and finds nothing there. Which shapes this recognizes at
/// all is an implementation's decision — this trait says only that the check
/// happens before the run, never that it always matches.
pub trait LinkLookup {
    fn resolve(&self, request: &str) -> Option<LookupCall>;

    /// The issue `request` links, where it links one this recognises — the
    /// same link [`resolve`](LinkLookup::resolve) would read. Spike 022,
    /// slice 4: a criterion read out of it says which issue it was.
    fn issue(&self, request: &str) -> Option<IssueAddress>;

    /// The call that answers when `issue` was last edited, one RFC 3339 instant
    /// on stdout. `None` where this does not recognise the address — an issue
    /// it never resolved. The rotation that asks is Fleet's (answer 5).
    fn edited(&self, issue: &IssueAddress) -> Option<LookupCall>;
}

/// Which issue a request linked: how a person reads it, and the forge's own
/// address for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IssueAddress {
    reference: String,
    url: String,
}

impl IssueAddress {
    pub fn at(reference: &str, url: &str) -> IssueAddress {
        IssueAddress {
            reference: String::from(reference),
            url: String::from(url),
        }
    }

    /// `armada#1162`: the repository and the number, which no branch or path
    /// reads as.
    pub fn reference(&self) -> &str {
        &self.reference
    }

    /// The address Bridge's main process may open.
    pub fn url(&self) -> &str {
        &self.url
    }
}
