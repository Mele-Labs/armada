//! `pr_mode:`, how this repository's pull requests are offered unless a
//! workflow's delivering step or the Job's own approval says otherwise.
//! `docs/concepts/landing.md`, *Pull request mode*.
//!
//! **Not a policy and not folded across Manifests**, for `merge_by`'s reason:
//! one Job opens one pull request, in one repository, and that repository's word
//! is its tier of the answer.

use core_model::PrMode;

use super::Manifest;
use crate::error::Refusal;
use crate::yaml::{self, Table};

const WORDS: &[(&str, PrMode)] = &[("ready", PrMode::Ready), ("draft", PrMode::Draft)];
const LEGAL: &[&str] = &["ready", "draft"];

/// What the key came to. **Absent is `None`, the repository deferring** to the
/// machine's default: `ready` written out is a statement that beats a machine
/// default of `draft`. A refused value reads as absent from here, for
/// `super::merge_by::read`'s reason: the refusal is already in `out`.
pub(super) fn read(top: &mut Table<'_>, out: &mut Vec<Refusal>) -> Option<PrMode> {
    top.optional("pr_mode")
        .and_then(|value| yaml::word("pr_mode", value, WORDS, LEGAL, LEGAL, out))
}

impl Manifest {
    /// How this repository offers a pull request, where it says. **Read through
    /// the live cell**, for `Manifest::merge_by`'s reason: it is asked at the
    /// approval, and a person changing it is owed the next Job's answer rather
    /// than the next restart's.
    pub fn pr_mode(&self) -> Option<PrMode> {
        self.live.read().pr_mode
    }
}
