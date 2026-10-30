//! Which rules apply to one repository, from Kit's and the repository's own.
//!
//! **Resolved as lifecycles are**, by the same reading ([`crate::resolving`]): the narrower scope
//! wins, then yours over the repository's, the loser is still listed and marked, and a Workspace's
//! rules committed to its home are read by its members from there. **Armada carries no rule**, so
//! with no file anywhere nothing applies and the engine plans nothing.
//!
//! **A rule that names a lifecycle or a state the lifecycles that apply here lack is set aside**,
//! with the sentence, rather than loaded to wait on a fact nobody will ever see.

use std::path::{Path, PathBuf};

use core_model::rule::{Rule, RuleId};
use core_model::{Lifecycle, LifecycleScope, ManifestId, PieceSource};

use crate::piece::{LeftOut, Written};
use crate::resolving::{resolve, Rank, Resolved, Scoped};
use crate::workspace::Workspaces;

use super::parse;

impl Scoped for Rule {
    type Id = RuleId;

    fn id(&self) -> &RuleId {
        Rule::id(self)
    }

    fn id_text(&self) -> &str {
        Rule::id(self).as_str()
    }

    fn scope(&self) -> &LifecycleScope {
        Rule::scope(self)
    }
}

/// One rule that applies, and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFile {
    rule: Rule,
    rank: Rank,
    path: PathBuf,
    overridden_by: Option<Rank>,
    hides: Option<PieceSource>,
}

impl RuleFile {
    pub fn rule(&self) -> &Rule {
        &self.rule
    }

    pub fn rank(&self) -> Rank {
        self.rank
    }

    /// Where it was read.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The rank of the one that runs instead of this. Absent on the one that runs.
    pub fn overridden_by(&self) -> Option<Rank> {
        self.overridden_by
    }

    /// The place whose file this one replaced because it says something else.
    pub fn hides(&self) -> Option<PieceSource> {
        self.hides
    }
}

impl From<Resolved<Rule>> for RuleFile {
    fn from(one: Resolved<Rule>) -> RuleFile {
        // Nothing is built in, so a rule that applies was always read from somewhere.
        RuleFile {
            rule: one.piece,
            rank: one.rank,
            path: one.path.unwrap_or_default(),
            overridden_by: one.overridden_by,
            hides: one.hides,
        }
    }
}

/// The rules that apply to one repository, and the files set aside.
#[derive(Debug, Default)]
pub struct Rules {
    files: Vec<RuleFile>,
    left_out: Vec<LeftOut>,
}

impl Rules {
    /// What applies to `repository`: every file in `written` whose scope names it, merged by id.
    /// `workspaces` says which Workspaces hold the repository, and `lifecycles` are the ones that
    /// apply to it, which a rule's names are checked against.
    pub fn applying_to(
        repository: &ManifestId,
        workspaces: &Workspaces,
        lifecycles: &[&Lifecycle],
        written: impl IntoIterator<Item = Written>,
    ) -> Rules {
        let read = |path: &Path, text: &str| {
            let rule = parse(path, text).map_err(|why| why.to_string())?;
            let unknown: Vec<String> = rule.unknown_in(lifecycles).iter().map(ToString::to_string).collect();
            match unknown.is_empty() {
                true => Ok(rule),
                false => Err(format!("{} {}", path.display(), unknown.join("; it "))),
            }
        };
        let (resolved, left_out) = resolve(repository, workspaces, Vec::new(), written, read);
        Rules { files: resolved.into_iter().map(RuleFile::from).collect(), left_out }
    }

    /// Every rule that applies: the one that runs for each id and each one it replaced.
    pub fn files(&self) -> &[RuleFile] {
        &self.files
    }

    /// The files set aside, and why.
    pub fn left_out(&self) -> &[LeftOut] {
        &self.left_out
    }

    /// One rule for each id: the one that runs.
    pub fn running(&self) -> impl Iterator<Item = &Rule> {
        self.files.iter().filter(|file| file.overridden_by.is_none()).map(|file| &file.rule)
    }

    /// The rule of this id that runs, if one applies.
    pub fn get(&self, id: &RuleId) -> Option<&Rule> {
        self.running().find(|rule| rule.id() == id)
    }

    /// The rules that run, copied: what a Job freezes when it starts, and what the engine is
    /// handed.
    pub fn frozen(&self) -> Vec<Rule> {
        self.running().cloned().collect()
    }
}
