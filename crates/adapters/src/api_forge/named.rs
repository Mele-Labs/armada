//! Which pull request a caller means. The API takes a number; `gh pr view` also takes the address
//! of one, which callers hand over when they hold the address and not the number.

use super::{numbered, ApiForge};
use crate::forge_address::pull_linked_in;

impl ApiForge {
    /// The number of the pull request `pull_request` names in the repository `in_repo` is a
    /// checkout of: a number, or the address of a pull request **in that repository**. Any other
    /// spelling, a branch or an address into another repository, is `gh`'s to read.
    pub(crate) fn pull_number(&self, in_repo: &str, pull_request: &str) -> Option<u64> {
        numbered(pull_request).or_else(|| {
            let repository = self.on_github(in_repo)?;
            pull_linked_in(pull_request, repository.address())
        })
    }
}
