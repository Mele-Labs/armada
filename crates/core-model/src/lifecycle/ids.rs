//! What a lifecycle, a state and a scope are called.
//!
//! **A lifecycle's id and a state's id are written by a person and read back in
//! `ends_on`** (`pr: approved`), so they are one word with no space and no colon,
//! checked once here and never again. A scope is the id of a Workspace (a set of repositories)
//! or of a repository, and this crate knows nothing more of it than that it is not blank.

use alloc::string::String;
use core::fmt;

/// An id that is not one word of lower case letters, digits and underscores starting with a letter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotAnId {
    written: String,
}

impl NotAnId {
    /// What was written.
    pub fn written(&self) -> &str {
        &self.written
    }
}

impl fmt::Display for NotAnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not an id. An id is lower case letters, digits and underscores, and starts with a letter",
            self.written
        )
    }
}

impl NotAnId {
    /// The refusal for `written`, for an id of another kind that is spelled the same way.
    pub(crate) fn spelled(written: &str) -> NotAnId {
        NotAnId { written: String::from(written) }
    }
}

impl core::error::Error for NotAnId {}

pub(crate) fn is_an_id(written: &str) -> bool {
    let mut chars = written.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

macro_rules! an_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(alloc::string::String);

        impl $name {
            /// The id, or the refusal naming what was written.
            pub fn new(written: &str) -> Result<$name, $crate::lifecycle::NotAnId> {
                match $crate::lifecycle::is_an_id(written) {
                    true => Ok($name(alloc::string::String::from(written))),
                    false => Err($crate::lifecycle::NotAnId::spelled(written)),
                }
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

pub(crate) use an_id;

an_id! {
    /// Which lifecycle: `ticket`, `pr`, `deploy`, `job`, or one a person defines.
    LifecycleId
}
an_id! {
    /// Which state of one lifecycle: `in_review`, `approved`.
    StateId
}

/// The id of a Workspace or a repository, which is all a lifecycle says of the place it applies to.
///
/// **Workspace here is the set of repositories that build one product**, and not the Manifest's
/// monorepo unit that `config` also calls a workspace.
///
/// **By id and not by value**: the Workspace is a noun of its own, in modules of its own, and a
/// lifecycle that held one would be a lifecycle that could not be read before it existed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(String);

impl ScopeId {
    /// The id trimmed, or the refusal for one with nothing in it.
    pub fn new(written: &str) -> Result<ScopeId, BlankScope> {
        match written.trim() {
            "" => Err(BlankScope),
            trimmed => Ok(ScopeId(String::from(trimmed))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A scope id with nothing in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankScope;

impl fmt::Display for BlankScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a scope is the id of a set of repositories or of one repository, and blank is neither")
    }
}

impl core::error::Error for BlankScope {}
