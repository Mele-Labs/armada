//! What this fake's forge does when asked to merge, and the numbering of the trees a merge makes.

use adapter_traits::NotMerged;

/// The branch's tree, and the one before the last merge into it, numbered.
#[derive(Debug, Default)]
pub(super) struct Trees {
    pub(super) now: u64,
    pub(super) was: u64,
    pub(super) made: u64,
}

impl Trees {
    pub(super) fn merged_into(&mut self) {
        self.made += 1;
        self.was = self.now;
        self.now = self.made;
    }

    pub(super) fn now(&self) -> String {
        format!("{:040x}", self.now)
    }
}

/// What this fake's forge does when asked to merge.
///
/// **A refusal is a value a test writes out**, not a string it matches on: the
/// kinds are what a caller acts on differently, and a fake that answered them
/// all with one sentence could not exercise that at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Merging {
    /// The forge takes it. **The default**, because a test about the press is
    /// about what follows a merge.
    #[default]
    Takes,
    /// Somebody had already merged it.
    AlreadyMerged,
    /// It would not, and which kind of would-not it was.
    Refuses(NotMerged),
}
