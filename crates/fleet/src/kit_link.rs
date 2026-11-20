//! Kit over the Mac link: each Mac offers the pieces of the kinds it has switched on, and takes from another Mac's
//! offer what changed there only since the two last agreed. `docs/concepts/kit.md`, *Over the Mac link*.
//!
//! **The last-agreed marker.** For each item and each linked Mac, this Mac keeps the hash of the item's canonical
//! text both Macs held when they last agreed on it (`kit_link_agreed`), and each piece the other Mac offers carries
//! that Mac's own marker with this one. Against one Mac, `h` this Mac's hash, `t` the other's, `a` this Mac's marker
//! and `b` the other's:
//!
//! | Here, there | Markers | Mark | What happens |
//! | --- | --- | --- | --- |
//! | `h == t` | any | same | `a` becomes `h` |
//! | `h != t` | `a` or `b` is `h`, neither is `t` | differs | there changed only: taken here, `a` becomes `t` |
//! | `h != t` | `a` or `b` is `t`, neither is `h` | differs | here changed only: the other Mac takes it |
//! | `h != t` | neither names either, or one names each | clash | held: each keeps its own until he picks |
//! | here only | any | new here | the other Mac takes it |
//! | there only | none | only there | new there: taken here |
//! | there only | some | only there | deleted here: not brought back |
//!
//! **Why both markers.** One Mac takes the other's edit and records it before the other has read that it did; an
//! edit made there in between moved past `a` alone, and would read as a clash. `b` names this Mac's text, so it is
//! the other's change only.
//!
//! **Each Mac writes only to itself.** Both Gateways follow each other, so each Mac takes from the other's offer on
//! its own exchange, and the two markers meet. **A deletion does not cross**: a piece one Mac took away is marked
//! on the other and left. **Settling a clash** records the hash of the side not kept as the marker, on both Macs, so
//! the kept side reads as the only change and crosses to the Mac that did not have it.
//!
//! Everything here is a function of what was read, so the rules run without a Fleet. [`serving`] reads, writes and
//! speaks for Fleet.

mod serving;

pub(crate) use serving::{SettleSent, Told};

use std::collections::{BTreeMap, BTreeSet};

use ipc::{HomeItem, HomeKind, Instant, KitAgainst, KitLinkRow, KitMark, KitOffer, KitPiece, KitSide};
use sha2::{Digest, Sha256};

use crate::home::compare::{canonical, mod_kind_of, shown};
use crate::home::pieces::{plain, Piece};
use crate::home::Syncs;

/// An item by its kind, a memory note's repository, and its name: how two Macs name the same item.
pub(crate) type Key = (HomeKind, Option<String>, String);

/// One of this Mac's pieces, read for a crossing.
#[derive(Clone, Debug)]
pub(crate) struct Mine {
    pub(crate) kind: HomeKind,
    pub(crate) piece: Piece,
    pub(crate) hash: String,
    pub(crate) changed_at: Option<Instant>,
    /// Why it does not leave this Mac: a credential, naming the host, or a mod not valid here.
    pub(crate) refused: Option<String>,
}

impl Mine {
    pub(crate) fn key(&self) -> Key {
        (self.kind, self.piece.repository.clone(), self.piece.name.clone())
    }
}

/// A linked Mac, as this Mac last heard it.
#[derive(Clone, Debug)]
pub(crate) struct Other {
    pub(crate) mac: String,
    pub(crate) offer: KitOffer,
    pub(crate) agreed: BTreeMap<Key, String>,
}

impl Other {
    /// Whether it sends `key`: its kind is on there, or the item is.
    fn sends(&self, key: &Key) -> bool {
        let (kind, repository, name) = key;
        self.offer.kinds.contains(kind)
            || self.offer.items.iter().any(|item| item.kind == *kind && item.name == *name && item.repository == *repository)
    }

    fn piece(&self, key: &Key) -> Option<&KitPiece> {
        self.offer.pieces.iter().find(|piece| key_of(piece) == *key)
    }
}

pub(crate) fn key_of(piece: &KitPiece) -> Key {
    (piece.kind, piece.repository.clone(), piece.name.clone())
}

/// The hash of `piece`'s canonical text, the same on every Mac for the same text.
pub(crate) fn hash_of(kind: HomeKind, piece: &Piece) -> String {
    let mut hash = Sha256::new();
    for (path, text) in canonical(kind, piece) {
        hash.update(path.as_bytes());
        hash.update([0]);
        hash.update(text.as_bytes());
        hash.update([0]);
    }
    format!("{:x}", hash.finalize())
}

pub(crate) fn piece_of(sent: &KitPiece) -> Piece {
    Piece { name: sent.name.clone(), repository: sent.repository.clone(), files: sent.files.clone(), says: sent.says.clone(), source: sent.source.clone() }
}

/// What this Mac sends: the kinds and items it has on, and each piece of them that may leave it, with `agreed`, its
/// markers with the Mac that asked.
pub(crate) fn offer(name: &str, syncs: &Syncs, mine: &[Mine], agreed: &BTreeMap<Key, String>) -> KitOffer {
    let pieces = mine
        .iter()
        .filter(|one| one.refused.is_none() && syncs.takes(one.kind, &one.piece.name, one.piece.repository.as_deref()))
        .map(|one| KitPiece {
            kind: one.kind,
            name: one.piece.name.clone(),
            repository: one.piece.repository.clone(),
            files: one.piece.files.clone(),
            says: one.piece.says.clone(),
            source: one.piece.source.clone(),
            hash: one.hash.clone(),
            changed_at: one.changed_at.clone(),
            agreed: agreed.get(&one.key()).cloned(),
        })
        .collect();
    KitOffer { name: name.to_string(), kinds: syncs.kinds.clone(), items: syncs.items.clone(), pieces }
}

/// A piece another Mac sent that may be put here: **its names come from another machine, so none is trusted to be
/// a path** until it is one segment, and every file under it stays under it.
pub(crate) fn fit_to_put(sent: &KitPiece) -> bool {
    let named = sent.kind == HomeKind::Allowlist || plain(&sent.name);
    let repository = sent.repository.as_deref().is_none_or(plain);
    let files = sent.files.keys().all(|path| !path.starts_with('/') && path.split('/').all(|part| !part.is_empty() && part != "." && part != ".."));
    named && repository && files
}

/// How one item stands against one Mac, from the three hashes, and whether to take theirs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Judged {
    pub(crate) mark: KitMark,
    pub(crate) take: bool,
    /// The marker to record now, where it moves without a take.
    pub(crate) agree: Option<String>,
}

/// `agreed` is this Mac's marker with the other, `theirs` the other's marker with this one, as its offer carried it.
pub(crate) fn judged(here: Option<&str>, there: Option<&str>, agreed: Option<&str>, theirs: Option<&str>) -> Judged {
    let (mark, take, agree) = match (here, there) {
        (Some(h), Some(t)) if h == t => (KitMark::Same, false, (agreed != Some(h)).then(|| h.to_string())),
        (Some(h), Some(t)) => {
            // Either Mac's marker naming a side's text says that side has not moved since they agreed.
            let unmoved_here = agreed == Some(h) || theirs == Some(h);
            let unmoved_there = agreed == Some(t) || theirs == Some(t);
            match (unmoved_here, unmoved_there) {
                (true, false) => (KitMark::Differs, true, None),
                (false, true) => (KitMark::Differs, false, None),
                _ => (KitMark::Clash, false, None),
            }
        }
        (Some(_), None) => (KitMark::NewHere, false, None),
        (None, Some(_)) => (KitMark::OnlyThere, agreed.is_none() && theirs.is_none(), None),
        (None, None) => (KitMark::Same, false, None),
    };
    Judged { mark, take, agree }
}

/// A piece to put here from Mac `mac`, over this Mac's own where it has one.
#[derive(Clone, Debug)]
pub(crate) struct Take {
    pub(crate) mac: String,
    pub(crate) kind: HomeKind,
    pub(crate) piece: Piece,
    pub(crate) replacing: Option<Piece>,
    pub(crate) hash: String,
}

/// What an exchange does: the pieces to put here, and the markers that move without one.
#[derive(Clone, Debug, Default)]
pub(crate) struct Plan {
    pub(crate) takes: Vec<Take>,
    pub(crate) agrees: Vec<(String, Key, String)>,
}

/// Every key this Mac takes, here or in a linked Mac's offer, in [`HomeKind::ALL`]'s order and then by name.
fn keys(syncs: &Syncs, mine: &[Mine], others: &[Other]) -> Vec<Key> {
    let order = |kind: &HomeKind| HomeKind::ALL.iter().position(|one| one == kind).unwrap_or(usize::MAX);
    let mut keys: BTreeSet<(usize, Option<String>, String)> = BTreeSet::new();
    let theirs = others.iter().flat_map(|other| other.offer.pieces.iter().filter(|piece| other.sends(&key_of(piece))).map(key_of));
    for (kind, repository, name) in mine.iter().map(Mine::key).chain(theirs) {
        if syncs.takes(kind, &name, repository.as_deref()) {
            keys.insert((order(&kind), repository, name));
        }
    }
    keys.into_iter().filter_map(|(at, repository, name)| Some((*HomeKind::ALL.get(at)?, repository, name))).collect()
}

/// What to take from `others` and which markers move. **An item is taken from one Mac at most** in one exchange; a
/// second Mac's newer version is judged again on the next.
pub(crate) fn plan(syncs: &Syncs, mine: &[Mine], others: &[Other]) -> Plan {
    let mut plan = Plan::default();
    for key in keys(syncs, mine, others) {
        let here = mine.iter().find(|one| one.key() == key);
        for other in others.iter().filter(|other| other.sends(&key)) {
            let there = other.piece(&key);
            let judged = judged(here.map(|one| one.hash.as_str()), there.map(|one| one.hash.as_str()), other.agreed.get(&key).map(String::as_str), there.and_then(|one| one.agreed.as_deref()));
            if let Some(hash) = judged.agree {
                plan.agrees.push((other.mac.clone(), key.clone(), hash));
            }
            // A server's key does not cross, so one is never put here: it is added in Kit with its key.
            let Some(there) = there.filter(|there| judged.take && key.0 != HomeKind::McpServers && fit_to_put(there)) else { continue };
            if plan.takes.iter().all(|one| (one.kind, one.piece.repository.clone(), one.piece.name.clone()) != key) {
                let replacing = here.map(|one| one.piece.clone());
                plan.takes.push(Take { mac: other.mac.clone(), kind: key.0, piece: piece_of(there), replacing, hash: there.hash.clone() });
            }
        }
    }
    plan
}

/// One Mac's version as a clash or a difference shows it.
fn side(mac_name: &str, piece: &Piece, changed_at: Option<&Instant>) -> KitSide {
    KitSide { mac_name: mac_name.to_string(), text: shown(piece), changed_at: changed_at.cloned() }
}

/// Every row, marked against each linked Mac that sends it. `here_name` is this Mac's name.
pub(crate) fn rows(here_name: &str, syncs: &Syncs, mine: &[Mine], others: &[Other]) -> Vec<KitLinkRow> {
    keys(syncs, mine, others)
        .into_iter()
        .map(|key| {
            let here = mine.iter().find(|one| one.key() == key);
            let mut theirs = None;
            let against = others
                .iter()
                .filter(|other| other.sends(&key))
                .map(|other| {
                    let there = other.piece(&key);
                    theirs = theirs.or(there);
                    let judged = judged(here.map(|one| one.hash.as_str()), there.map(|one| one.hash.as_str()), other.agreed.get(&key).map(String::as_str), there.and_then(|one| one.agreed.as_deref()));
                    let both = matches!(judged.mark, KitMark::Differs | KitMark::Clash);
                    KitAgainst {
                        mac: other.mac.clone(),
                        mark: judged.mark,
                        here: here.filter(|_| both).map(|one| side(here_name, &one.piece, one.changed_at.as_ref())),
                        there: there.filter(|_| both).map(|one| side(&other.offer.name, &piece_of(one), one.changed_at.as_ref())),
                    }
                })
                .collect();
            let (kind, repository, name) = key;
            let theirs = theirs.map(piece_of);
            let either = here.map(|one| &one.piece).or(theirs.as_ref());
            KitLinkRow {
                kind,
                name,
                repository,
                says: either.and_then(|one| one.says.clone()),
                source: either.and_then(|one| one.source.clone()),
                key_here: (kind == HomeKind::McpServers).then_some(here.is_some()),
                mod_kind: (kind == HomeKind::Mods).then(|| here.and_then(|one| mod_kind_of(&one.piece)).or_else(|| theirs.as_ref().and_then(mod_kind_of))).flatten(),
                refused: here.and_then(|one| one.refused.clone()),
                against,
            }
        })
        .collect()
}

/// The item a key names on the wire.
pub(crate) fn item_of(key: &Key) -> HomeItem {
    HomeItem { kind: key.0, name: key.2.clone(), repository: key.1.clone() }
}
