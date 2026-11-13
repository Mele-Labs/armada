//! A Project is a set of members, each a whole repository or a monorepo folder inside one.
//!
//! **A Workspace keeps only its monorepo meaning now**: the folder of a repository that carries its
//! own `armada.yml`. A member may name one, by [`Folder`]. The owner decided this on 10 Nov 2026,
//! when a Project replaced what was called a Workspace here.
//!
//! A Project is a scope: it says which repositories a lifecycle (and a workflow and a rule) applies
//! to. A lifecycle names it by [`ScopeId`] and never holds it, so a lifecycle can be read before the
//! Project it names exists. **A new Project is made empty**, so it may hold no member at all.
//!
//! **A repository is named the way Fleet names one everywhere else**, by its [`ManifestId`].

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::lifecycle::ScopeId;
use crate::values::Values;
use crate::workflow::{ManifestId, Title};

/// A path inside a repository, `/`-separated and relative to its root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Folder(String);

impl Folder {
    /// A folder, or what is wrong with the path. **Nothing is trimmed**: a path with a leading or
    /// trailing `/` is a mistake to say, not one to fix.
    pub fn new(written: &str) -> Result<Folder, MalformedFolder> {
        if written.is_empty() {
            return Err(MalformedFolder::Empty);
        }
        if written.starts_with('/') || written.ends_with('/') {
            return Err(MalformedFolder::EdgeSlash);
        }
        if written.split('/').any(|segment| segment == "." || segment == ".." || segment.is_empty()) {
            return Err(MalformedFolder::Segment);
        }
        Ok(Folder(String::from(written)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a folder cannot be.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MalformedFolder {
    Empty,
    /// It starts or ends with `/`.
    EdgeSlash,
    /// A segment is `.`, `..` or nothing (`a//b`), so the path does not stay inside the repository.
    Segment,
}

impl fmt::Display for MalformedFolder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MalformedFolder::Empty => "a folder is a path inside the repository, and blank is not one",
            MalformedFolder::EdgeSlash => "a folder does not start or end with `/`",
            MalformedFolder::Segment => "a folder has no `.`, `..` or empty segment, so it stays inside the repository",
        })
    }
}

impl core::error::Error for MalformedFolder {}

/// One member: a whole repository, or a folder of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    repository: ManifestId,
    folder: Option<Folder>,
}

impl Member {
    pub fn whole(repository: ManifestId) -> Member {
        Member { repository, folder: None }
    }

    pub fn folder(repository: ManifestId, folder: Folder) -> Member {
        Member { repository, folder: Some(folder) }
    }

    pub fn repository(&self) -> &ManifestId {
        &self.repository
    }

    /// The folder of the repository, absent for the whole of it.
    pub fn folder_of(&self) -> Option<&Folder> {
        self.folder.as_ref()
    }
}

/// A set of members under one id and one name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    id: ScopeId,
    name: Title,
    members: Vec<Member>,
    values: Values,
    shared_with_macs: bool,
}

impl Project {
    /// A Project, or the first thing wrong with it.
    pub fn declared(id: ScopeId, name: Title, members: Vec<Member>) -> Result<Project, MalformedProject> {
        for (n, member) in members.iter().enumerate() {
            let earlier = &members[..n];
            if earlier.contains(member) {
                return Err(MalformedProject::MemberTwice { repository: member.repository.clone(), folder: member.folder.clone() });
            }
            let beside = |other: &Member| {
                other.repository == member.repository && (other.folder.is_none() != member.folder.is_none())
            };
            if earlier.iter().any(beside) {
                return Err(MalformedProject::WholeBesideFolder { repository: member.repository.clone() });
            }
            if member.repository.as_str() == id.as_str() {
                return Err(MalformedProject::NamedLikeItsRepository { repository: member.repository.clone() });
            }
        }
        Ok(Project { id, name, members, values: Values::default(), shared_with_macs: false })
    }

    /// The same Project setting `values`, which replace what it set whole.
    pub fn with_values(mut self, values: Values) -> Project {
        self.values = values;
        self
    }

    /// The same Project shared with his other Macs, or not.
    pub fn with_shared_with_macs(mut self, shared: bool) -> Project {
        self.shared_with_macs = shared;
        self
    }

    /// Whether anything of its Jobs may leave this Mac for his other Macs. **Off until he turns it on.**
    pub fn shared_with_macs(&self) -> bool {
        self.shared_with_macs
    }

    /// What the Project sets for how work behaves. Nothing set is the empty [`Values`].
    pub fn values(&self) -> &Values {
        &self.values
    }

    /// The id a lifecycle's `scope` names.
    pub fn id(&self) -> &ScopeId {
        &self.id
    }

    /// What a person reads for it.
    pub fn name(&self) -> &Title {
        &self.name
    }

    /// The members, in the order the Project lists them.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// The repositories its members name, each once, in the order they first appear.
    pub fn repositories(&self) -> Vec<&ManifestId> {
        let mut seen: Vec<&ManifestId> = Vec::new();
        for member in &self.members {
            if !seen.contains(&&member.repository) {
                seen.push(&member.repository);
            }
        }
        seen
    }

    /// Whether any member names `repository`, whole or by a folder.
    pub fn holds(&self, repository: &ManifestId) -> bool {
        self.members.iter().any(|member| &member.repository == repository)
    }
}

/// What a Project cannot be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MalformedProject {
    /// The same member is listed twice.
    MemberTwice { repository: ManifestId, folder: Option<Folder> },
    /// A whole repository is listed beside a folder of it, and the folder is already inside the whole.
    WholeBesideFolder { repository: ManifestId },
    /// Its id is a repository's too. **A scope names a repository or a Project by one id**, and a
    /// Project that shares its own member's would leave a lifecycle scoped to it unable to say
    /// which of the two it meant.
    NamedLikeItsRepository { repository: ManifestId },
}

impl fmt::Display for MalformedProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MalformedProject::MemberTwice { repository, folder: None } => {
                write!(f, "`{}` is listed twice", repository.as_str())
            }
            MalformedProject::MemberTwice { repository, folder: Some(folder) } => {
                write!(f, "`{}` folder `{}` is listed twice", repository.as_str(), folder.as_str())
            }
            MalformedProject::WholeBesideFolder { repository } => write!(
                f,
                "`{}` is listed whole and also by a folder, and the folder is already in the whole",
                repository.as_str()
            ),
            MalformedProject::NamedLikeItsRepository { repository } => write!(
                f,
                "its id is also the id of `{}`, a repository it holds, and a scope could not say which it meant",
                repository.as_str()
            ),
        }
    }
}

impl core::error::Error for MalformedProject {}

#[cfg(test)]
mod tests;
