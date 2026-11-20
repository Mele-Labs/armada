//! Kit's pieces on this Mac, kind by kind, in the one form every Mac reads and writes them: what crosses to his
//! other Macs over the link (`crate::kit_link`), and where a piece from one is put here. `docs/concepts/kit.md`,
//! *Over the Mac link*.
//!
//! **The shared home is retired.** Fleet no longer clones, pulls, pushes or searches for the `armada-home`
//! repository, and leaves it as it is; [`serving`] answers its routes for Classic Bridge with an empty read and a
//! refusal that names the link.
//!
//! **Nothing secret leaves**: a server travels as its name and host, and a piece whose text carries a credential in
//! an address is refused, `secrets`' reason.
//!
//! Everything here is a function over [`Places`], so the claim runs on folders held in memory.

pub(crate) mod compare;
pub(crate) mod pieces;
pub(crate) mod secrets;
mod serving;

use adapter_traits::{HarnessHome, KeptFolder};
use core_model::KitServer;
use ipc::{HomeItem, HomeKind};

pub use serving::HomeFittings;
pub(crate) use serving::HomeHeld;

use pieces::Piece;

/// A repository this machine serves, which a memory note is matched by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    /// The folder's own name, which is what names it on every machine.
    pub name: String,
    pub root: String,
}

/// Where each kind is on this machine.
pub struct Places<'a> {
    /// The machine's mods folder.
    pub mods: &'a dyn KeptFolder,
    /// Kit's home: its workflows and its allowlist.
    pub kit: &'a dyn KeptFolder,
    pub harness: &'a dyn HarnessHome,
    /// Kit's own servers, as the store holds them.
    pub servers: &'a [KitServer],
    pub repositories: &'a [Repository],
}

/// What crosses: the kinds switched on, and each item switched on on its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Syncs {
    pub kinds: Vec<HomeKind>,
    pub items: Vec<HomeItem>,
}

impl Syncs {
    /// The kinds a read looks at: each one on, and each with an item on.
    pub(crate) fn looked_at(&self) -> impl Iterator<Item = HomeKind> + '_ {
        HomeKind::ALL.iter().copied().filter(|kind| self.kinds.contains(kind) || self.items.iter().any(|item| item.kind == *kind))
    }

    /// Whether the piece `name` of `kind` crosses: its whole kind is on, or it is.
    pub(crate) fn takes(&self, kind: HomeKind, name: &str, repository: Option<&str>) -> bool {
        self.kinds.contains(&kind)
            || self.items.iter().any(|item| item.kind == kind && item.name == name && item.repository.as_deref() == repository)
    }
}

/// Why a piece did not move.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Its text carries a credential, and it stays on this machine.
    Secret(String),
    /// It cannot go where it was sent.
    Cannot(String),
}

impl Refused {
    pub fn said(&self) -> String {
        match self {
            Refused::Secret(said) | Refused::Cannot(said) => said.clone(),
        }
    }
}

/// Each piece's text, so none of it carries a credential out.
pub(crate) fn kept_here(piece: &Piece) -> Result<(), Refused> {
    let texts = std::iter::once(piece.name.as_str()).chain(piece.files.values().map(String::as_str));
    match texts.filter_map(secrets::leak).next() {
        Some(why) => {
            // An allowlist line is its own name, and the credential would ride the refusal out.
            let shown = if secrets::leak(&piece.name).is_some() { "It" } else { piece.name.as_str() };
            Err(Refused::Secret(format!("{shown} stays on this Mac: {why}")))
        }
        None => Ok(()),
    }
}
