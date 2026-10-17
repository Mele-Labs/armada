//! The two Manifest policies an `armada.yml` writes: whether a machine may land
//! work, and whether a person signs off on a review step.
//!
//! **Values, not gates.** [`AutoMerge`] and [`ReviewGate`] are what an
//! `armada.yml` writes; they are not [`AdvanceGate`](crate::AdvanceGate)
//! variants and never become one. Both settings are `Live`, so the answer is
//! true only until somebody saves the file.
//!
//! **No registry rows, and that is the registry's own decision.**
//! Nothing renders an [`AutoMerge`] or a [`ReviewGate`], so neither is a
//! vocabulary the surface speaks. `crates/config/settings.toml` is where the
//! values are written down, one row each.

/// Whether a machine may land work, as one `armada.yml` says it.
///
/// **The default is [`Never`](AutoMerge::Never) and it is not an absence.** A
/// repository that says nothing is a repository where no machine decides that
/// work lands, which is the answer `crates/fleet/src/merging.rs` was written
/// under: a person presses, and Fleet performs the merge so the Checks over the
/// merged tree run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoMerge {
    /// No machine decides. The step holds and a person answers it.
    #[default]
    Never,
    /// A machine may decide, once **the forge's** checks have all passed.
    ///
    /// **Not Armada's Checks, and the name is deliberately the ambiguous one.**
    /// It sits in the same file as a `checks:` block that means something else,
    /// and it was named for the person writing it rather than the person
    /// reading it beside that block: on a forge the word for these is checks,
    /// and calling them tests was wrong because one of them is as likely to be
    /// a lint or a build. The cost is that this doc comment and
    /// `docs/concepts/manifest.md` carry the disambiguation instead of the
    /// name doing it.
    ///
    /// `adapter_traits::WhatTheForgeRan` is the reading it names — the
    /// repository's own automation, run against the branch under review — and a
    /// Check declared in `armada.yml` has already run at the gate this policy
    /// is read at. Totalling the two would say a gate had held that never ran.
    ///
    /// **A repository whose forge runs nothing never merges under this.**
    /// `NothingRan` is not `AllPassed`, which is the cautious reading and the
    /// one the fold everywhere else in this file takes.
    ChecksPass,
    /// A machine may decide, whatever ran.
    Always,
}

impl AutoMerge {
    /// Every value, for a refusal that names what the file could have said.
    pub const ALL: &'static [AutoMerge] =
        &[AutoMerge::Never, AutoMerge::ChecksPass, AutoMerge::Always];

    /// Exactly as `armada.yml` writes it. **Hyphenated in the middle value**,
    /// which is `docs/concepts/manifest.md`'s spelling and the one the settings
    /// row carries; the underscored gate words next to it are a different
    /// vocabulary and are not made to match.
    pub fn as_written(&self) -> &'static str {
        match self {
            AutoMerge::Never => "never",
            AutoMerge::ChecksPass => "checks-pass",
            AutoMerge::Always => "always",
        }
    }

    /// Read one back. `None` is a word the policy has no value for, which the
    /// caller refuses rather than defaults — a mistyped `auto_merge: nevr`
    /// silently meaning `never` would be right by luck here and wrong the day
    /// somebody mistypes `always`.
    pub fn from_written(value: &str) -> Option<AutoMerge> {
        AutoMerge::ALL
            .iter()
            .copied()
            .find(|policy| policy.as_written() == value)
    }
}

/// Whether a person signs off on a workflow's review step, as one `armada.yml`
/// says it.
///
/// **The default is [`HumanAlways`](ReviewGate::HumanAlways)**, which
/// `crates/config/settings.toml` states outright and `docs/concepts/manifest.md`
/// repeats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewGate {
    /// A person answers, exactly as
    /// [`AdvanceGate::HumanAlways`](crate::AdvanceGate::HumanAlways) does.
    #[default]
    HumanAlways,
    /// The step advances where the mechanical tier held and the Judge did not
    /// refuse — [`AdvanceGate::AutoIfJudgePasses`](crate::AdvanceGate::AutoIfJudgePasses)'s
    /// meaning, reached through a policy instead of through the file.
    ///
    /// **It says nothing about a step that asks the Judge nothing.** A
    /// judgeless step resolving to this would advance on the mechanical tier
    /// alone while reading as judged, which is the disagreement
    /// `config::workflow::step` refuses when the gate is a plain word and
    /// cannot refuse when it is a policy. `fleet::gate` is where that meets a
    /// step's declaration.
    AutoIfJudgePasses,
}

impl ReviewGate {
    /// Every value, for [`AutoMerge::ALL`]'s reason.
    pub const ALL: &'static [ReviewGate] =
        &[ReviewGate::HumanAlways, ReviewGate::AutoIfJudgePasses];

    /// Exactly as `armada.yml` writes it. **The same two words the gate uses**,
    /// and deliberately so: a repository setting `auto_if_judge_passes` is
    /// asking for the gate that word already names, so one spelling covers
    /// both.
    pub fn as_written(&self) -> &'static str {
        match self {
            ReviewGate::HumanAlways => "human_always",
            ReviewGate::AutoIfJudgePasses => "auto_if_judge_passes",
        }
    }

    /// Read one back, for [`AutoMerge::from_written`]'s reason.
    pub fn from_written(value: &str) -> Option<ReviewGate> {
        ReviewGate::ALL
            .iter()
            .copied()
            .find(|policy| policy.as_written() == value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The spellings round-trip, and a word neither policy has is `None`
    /// rather than the default — the whole reason `from_written` answers with
    /// an `Option`.
    #[test]
    fn a_word_the_policy_has_no_value_for_is_refused_rather_than_defaulted() {
        for policy in AutoMerge::ALL {
            assert_eq!(AutoMerge::from_written(policy.as_written()), Some(*policy));
        }
        for policy in ReviewGate::ALL {
            assert_eq!(ReviewGate::from_written(policy.as_written()), Some(*policy));
        }
        assert_eq!(AutoMerge::from_written("tests_pass"), None);
        assert_eq!(AutoMerge::from_written("human_always"), None);
        assert_eq!(ReviewGate::from_written("auto"), None);
        assert_eq!(ReviewGate::from_written("never"), None);
    }
}
