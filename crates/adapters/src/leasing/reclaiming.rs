//! What a slot holds after the pull request from its branch merged, read before Fleet gives it
//! back. A sibling of `leasing.rs` for the reason `parking.rs` is: that file stays whole.

use adapter_traits::SlotAfterMerge;

use super::git::{dirty, git_ok};
use super::{checked_out, Pool};

impl Pool {
    /// Whether slot `number`, leased on `merged`, holds anything new. **Read only.**
    ///
    /// Idle is the checkout still on `merged`, or detached on a commit the base has, clean, with
    /// every commit on a remote or the base. Another branch checked out is new work and is left
    /// alone. A dirty tree or commits no remote or
    /// base has are named, and the slot is not given back. A slot git cannot be asked about is
    /// left as it is.
    pub fn after_merge(&self, number: usize, merged: &str) -> SlotAfterMerge {
        let at = self.path_of(number);
        let head = checked_out(&at);
        if head.as_deref().is_some_and(|on| on != merged) {
            return SlotAfterMerge::Moved;
        }
        let unlanded = self.unlanded(&at, "HEAD");
        if head.is_none()
            && unlanded == 0
            && !git_ok(&at, &["merge-base", "--is-ancestor", "HEAD", &self.base_ref()])
        {
            return SlotAfterMerge::Moved;
        }
        match dirty(&at) {
            Err(_) => return SlotAfterMerge::Moved,
            Ok(files) if !files.is_empty() => return SlotAfterMerge::Dirty(files),
            Ok(_) => {}
        }
        match unlanded {
            0 => SlotAfterMerge::Idle,
            commits => SlotAfterMerge::Unpushed(commits),
        }
    }
}
