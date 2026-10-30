//! What a rule and the words in it are called.

use alloc::string::String;
use core::fmt;

use crate::lifecycle::an_id;

an_id! {
    /// Which rule. **Written by a person and read back by the editor**, so it is one word as a
    /// lifecycle's id is. Where two rules share one, the narrower scope wins, then yours over the
    /// repository's.
    RuleId
}

/// Text a rule carries to something else: a ticket's assignee, a template, a prompt, a command's
/// name. **Never blank**, since an act with nothing to say has nothing to do.
///
/// Stored trimmed, as a title is. What the words mean is the act's, and the engine plans with
/// them as written: a template is filled in when the act is done, not when it is planned.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Words(String);

impl Words {
    pub fn new(written: &str) -> Result<Words, BlankWords> {
        match written.trim() {
            "" => Err(BlankWords),
            trimmed => Ok(Words(String::from(trimmed))),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Words with nothing in them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlankWords;

impl fmt::Display for BlankWords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an act is told what to do in words, and blank says nothing")
    }
}

impl core::error::Error for BlankWords {}
