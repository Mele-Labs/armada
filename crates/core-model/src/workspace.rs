//! A Workspace is a set of repositories that build one product, as a VS Code workspace is.
//!
//! **It is not the Manifest's monorepo unit**, the one `armada.yml` inside a repository that
//! `config` also calls a workspace. The owner kept both senses on 29 Oct; they share a word and no
//! code, and this module is the first sense only.
//!
//! A Workspace is a scope: it says which repositories a lifecycle (and later a workflow and a rule)
//! applies to. A lifecycle names it by [`ScopeId`] and never holds it, so a lifecycle can be read
//! before the Workspace it names exists.
//!
//! **A repository is named the way Fleet names one everywhere else**, by its [`ManifestId`].

use alloc::vec::Vec;
use core::fmt;

use crate::lifecycle::ScopeId;
use crate::workflow::{ManifestId, Title};

/// A set of repositories under one id and one title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    id: ScopeId,
    title: Title,
    repositories: Vec<ManifestId>,
}

impl Workspace {
    /// A Workspace, or the first thing wrong with it.
    pub fn declared(
        id: ScopeId,
        title: Title,
        repositories: Vec<ManifestId>,
    ) -> Result<Workspace, MalformedWorkspace> {
        if repositories.is_empty() {
            return Err(MalformedWorkspace::NoRepositories);
        }
        for (n, repository) in repositories.iter().enumerate() {
            if repositories[..n].contains(repository) {
                return Err(MalformedWorkspace::RepositoryTwice { repository: repository.clone() });
            }
            if repository.as_str() == id.as_str() {
                return Err(MalformedWorkspace::NamedLikeItsRepository { repository: repository.clone() });
            }
        }
        Ok(Workspace { id, title, repositories })
    }

    /// The id a lifecycle's `scope` names.
    pub fn id(&self) -> &ScopeId {
        &self.id
    }

    /// What a person reads for it.
    pub fn title(&self) -> &Title {
        &self.title
    }

    /// The repositories, in the order the Workspace lists them.
    pub fn repositories(&self) -> &[ManifestId] {
        &self.repositories
    }

    pub fn holds(&self, repository: &ManifestId) -> bool {
        self.repositories.contains(repository)
    }
}

/// What a Workspace cannot be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MalformedWorkspace {
    /// It holds nothing, so it scopes nothing.
    NoRepositories,
    /// A repository is listed twice.
    RepositoryTwice { repository: ManifestId },
    /// Its id is a repository's too. **A scope names a repository or a Workspace by one id**, and a
    /// Workspace that shares its own member's would leave a lifecycle scoped to it unable to say
    /// which of the two it meant.
    NamedLikeItsRepository { repository: ManifestId },
}

impl fmt::Display for MalformedWorkspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MalformedWorkspace::NoRepositories => {
                f.write_str("a Workspace holds at least one repository, and this holds none")
            }
            MalformedWorkspace::RepositoryTwice { repository } => {
                write!(f, "`{}` is listed twice", repository.as_str())
            }
            MalformedWorkspace::NamedLikeItsRepository { repository } => write!(
                f,
                "its id is also the id of `{}`, a repository it holds, and a scope could not say which it meant",
                repository.as_str()
            ),
        }
    }
}

impl core::error::Error for MalformedWorkspace {}

#[cfg(test)]
mod tests;
