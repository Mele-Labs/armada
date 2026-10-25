//! Kit against the shared home: each kind read on this machine and in the home's copy here,
//! compared row by row, and the three acts that settle a row. `docs/concepts/kit.md`, *The
//! shared home*.
//!
//! **A row that differs is never merged.** Bring and Add take a row that is on one side only;
//! a row on both that disagrees is refused by both, and only [`choose`] replaces one side with
//! the other. **Nothing secret leaves**: a server travels as its name and host, and a piece whose
//! text carries a credential in an address is refused, `secrets`' reason.
//!
//! **Each profile has its own home**, `<login>/armada-home` until it chooses another, and every act names the profile: a
//! machine can hold a personal profile and a work one. [`home_of`] is the one way to a home.
//!
//! Everything here is a function over [`Places`], so the claim runs on folders held in memory.
//! [`serving`] is the part that speaks for Fleet.

mod compare;
mod pieces;
pub(crate) mod secrets;
mod serving;

use std::collections::BTreeMap;
use std::sync::Arc;

use adapter_traits::{HarnessHome, HomeFailure, HomePlace, HomeRepositories, HomeRepository, KeptFolder};
use core_model::KitServer;
use ipc::{HomeKind, HomeKindSwitch, HomeKinds, HomeLeft, HomeNamed, HomeRowAt, HomeSide, SharedHomeRead};

pub use serving::HomeFittings;
pub(crate) use serving::HomeHeld;

use crate::profiles::Joined;
use pieces::Piece;

/// A repository this machine serves, which a memory note is matched by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repository {
    /// The folder's own name, which is what names it on every machine.
    pub name: String,
    pub root: String,
}

/// Where each kind is on this machine, and the home's copy here.
pub struct Places<'a> {
    pub home: &'a dyn HomeRepository,
    /// The machine's mods folder.
    pub mods: &'a dyn KeptFolder,
    /// Kit's home: its workflows and its allowlist.
    pub kit: &'a dyn KeptFolder,
    pub harness: &'a dyn HarnessHome,
    /// Kit's own servers, as the store holds them.
    pub servers: &'a [KitServer],
    pub repositories: &'a [Repository],
}

/// Why an act did not happen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The row is not where the act needs it.
    NotThere(String),
    /// It is on both sides and they disagree: only a chosen side settles it.
    Differs(String),
    /// Its text carries a credential, and it stays on this machine.
    Secret(String),
    /// It cannot go where it was sent.
    Cannot(String),
    /// The login is not a profile on this machine.
    NoProfile(String),
    /// The owner or the name chosen for the home is not one it can have.
    Choice(String),
    /// The home is made, or the forge has it, and a made home is not moved.
    Made(String),
    Home(HomeFailure),
}

impl Refused {
    pub fn said(&self) -> String {
        match self {
            Refused::NotThere(said)
            | Refused::Differs(said)
            | Refused::Secret(said)
            | Refused::Cannot(said)
            | Refused::NoProfile(said)
            | Refused::Choice(said)
            | Refused::Made(said) => said.clone(),
            Refused::Home(failure) => failure.said(),
        }
    }
}

/// `login`'s home, where `login` is one of this machine's `profiles`. **Asks the forge nothing**:
/// a login the forge here does not hold is refused by the first act on it, naming the login.
pub fn home_of(homes: &dyn HomeRepositories, login: &str, profiles: &[Joined]) -> Result<Arc<dyn HomeRepository>, Refused> {
    let profile = profile_of(login, profiles)?;
    homes.of(login, &owners_of(profile)).map_err(Refused::Home)
}

/// The profile `login` names, or the refusal that it is not one here.
pub fn profile_of<'a>(login: &str, profiles: &'a [Joined]) -> Result<&'a Joined, Refused> {
    profiles.iter().find(|profile| profile.login == login).ok_or_else(|| Refused::NoProfile(format!("{login} is not a profile on this machine")))
}

/// The owners a home of `profile`'s may be under: its login and its organisations.
pub fn owners_of(profile: &Joined) -> Vec<String> {
    std::iter::once(profile.login.clone()).chain(profile.orgs.iter().cloned()).collect()
}

/// The most characters a repository name holds on the forge.
const NAME_MOST: usize = 100;

/// Where `profile` chose to make its home, or why it cannot be there: the owner is the login or
/// one of its organisations, and the name is one the forge takes.
pub fn chosen(profile: &Joined, owner: &str, name: &str) -> Result<HomePlace, Refused> {
    let owner = owners_of(profile)
        .into_iter()
        .find(|it| it.eq_ignore_ascii_case(owner))
        .ok_or_else(|| Refused::Choice(format!("{owner} is neither {} nor one of its organisations", profile.login)))?;
    let plain = (1..=NAME_MOST).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        && name != "."
        && name != "..";
    match plain {
        true => Ok(HomePlace { owner, name: name.to_string() }),
        false => Err(Refused::Choice(format!(
            "{name:?} is not a name the forge takes: use 1 to {NAME_MOST} letters, digits, dots, underscores and hyphens"
        ))),
    }
}

/// Make the home at `place` where it does not exist and clone it, or pull the copy here; then write every
/// row only on this machine, of each kind in `on`, into the copy as [`add`] does, and push them
/// in one commit. A row `valid` or the home refuses stays here and is named in what this answers;
/// the others still go. `machine` names this one in the commit.
pub fn sync(
    places: &Places<'_>,
    on: &[HomeKind],
    machine: &str,
    place: &HomePlace,
    valid: &dyn Fn(HomeKind, &str) -> Result<(), Refused>,
) -> Result<Vec<HomeLeft>, Refused> {
    places.home.open(place).map_err(Refused::Home)?;
    let (mut paths, mut added, mut left) = (Vec::new(), Vec::new(), Vec::new());
    for kind in HomeKind::ALL.iter().copied().filter(|kind| on.contains(kind)) {
        let there = pieces::home(places, kind).pieces;
        let only_here = pieces::here(places, kind).pieces.into_iter().filter(|here| {
            !there.iter().any(|piece| piece.name == here.name && piece.repository == here.repository)
        });
        for here in only_here {
            let at = HomeRowAt { login: String::new(), kind, name: here.name.clone(), repository: here.repository.clone() };
            match valid(kind, &here.name).and_then(|()| staged(places, kind, &here, None)) {
                Ok(touched) => {
                    paths.extend(touched);
                    added.push(named(&at));
                }
                Err(refused) => left.push(HomeLeft { kind, name: at.name, repository: at.repository, why: refused.said() }),
            }
        }
    }
    if !added.is_empty() {
        // An allowlist line each rewrites the one file.
        paths.sort();
        paths.dedup();
        places.home.publish(&paths, &format!("Sync from {machine}: add {}", added.join(", "))).map_err(Refused::Home)?;
    }
    Ok(left)
}

fn named(at: &HomeRowAt) -> String {
    match &at.repository {
        Some(repository) => format!("{} {repository}/{}", at.kind.as_wire(), at.name),
        None => format!("{} {}", at.kind.as_wire(), at.name),
    }
}

fn find(pieces: Vec<Piece>, at: &HomeRowAt) -> Option<Piece> {
    let repository = at.repository.as_deref().filter(|_| at.kind == HomeKind::Memory);
    pieces.into_iter().find(|piece| piece.name == at.name && piece.repository.as_deref() == repository)
}

/// The home this machine holds a copy of, as `login/repository`.
fn home_named(home: &dyn HomeRepository) -> Option<HomeNamed> {
    let here = home.here()?;
    let (account, repository) = here.split_once('/')?;
    Some(HomeNamed { account: account.to_string(), repository: repository.to_string() })
}

/// Every row of every kind in `on`. **None at all where this machine holds no copy of the home**:
/// there is nothing to compare against.
pub fn read(places: &Places<'_>, on: &[HomeKind]) -> SharedHomeRead {
    let Some(home) = home_named(places.home) else {
        return SharedHomeRead { home: None, planned: None, rows: Vec::new(), unreadable: Vec::new() };
    };
    let (mut rows, mut unreadable) = (Vec::new(), Vec::new());
    for kind in HomeKind::ALL.iter().copied().filter(|kind| on.contains(kind)) {
        let (here, there) = (pieces::here(places, kind), pieces::home(places, kind));
        rows.extend(compare::rows(kind, &here.pieces, &there.pieces));
        unreadable.extend(here.unreadable.into_iter().chain(there.unreadable));
    }
    SharedHomeRead { home: Some(home), planned: None, rows, unreadable }
}

/// Each piece's text, so none of it carries a credential out.
fn kept_here(piece: &Piece) -> Result<(), Refused> {
    let texts = std::iter::once(piece.name.as_str()).chain(piece.files.values().map(String::as_str));
    match texts.filter_map(secrets::leak).next() {
        Some(why) => {
            // An allowlist line is its own name, and the credential would ride the refusal out.
            let shown = if secrets::leak(&piece.name).is_some() { "It" } else { piece.name.as_str() };
            Err(Refused::Secret(format!("{shown} stays on this machine: {why}")))
        }
        None => Ok(()),
    }
}

fn the_two(places: &Places<'_>, at: &HomeRowAt) -> (Option<Piece>, Option<Piece>) {
    (find(pieces::here(places, at.kind).pieces, at), find(pieces::home(places, at.kind).pieces, at))
}

/// Write `here` into the home's copy, refused where it carries a credential. The paths it touched.
fn staged(places: &Places<'_>, kind: HomeKind, here: &Piece, replacing: Option<&Piece>) -> Result<Vec<String>, Refused> {
    kept_here(here)?;
    pieces::put_home(places, kind, here, replacing)
}

fn published(places: &Places<'_>, at: &HomeRowAt, here: &Piece, replacing: Option<&Piece>, said: String) -> Result<(), Refused> {
    let paths = staged(places, at.kind, here, replacing)?;
    places.home.publish(&paths, &said).map_err(Refused::Home)
}

/// Put a row that is only on this machine into the home, and push it. `machine` names this one
/// in the commit.
pub fn add(places: &Places<'_>, at: &HomeRowAt, machine: &str) -> Result<(), Refused> {
    let (here, there) = the_two(places, at);
    let here = here.ok_or_else(|| Refused::NotThere(format!("{} is not on this machine", named(at))))?;
    match there {
        Some(there) if compare::same(at.kind, &here, &there) => Ok(()),
        Some(_) => Err(Refused::Differs(format!("{} is in the home already and differs: choose a side", named(at)))),
        None => published(places, at, &here, None, format!("Add {} from {machine}", named(at))),
    }
}

/// Bring a row that is only in the home to this machine. **Never over a file of its own**.
pub fn bring(places: &Places<'_>, at: &HomeRowAt) -> Result<(), Refused> {
    let (here, there) = the_two(places, at);
    let there = there.ok_or_else(|| Refused::NotThere(format!("{} is not in the home", named(at))))?;
    match here {
        Some(here) if compare::same(at.kind, &here, &there) => Ok(()),
        Some(_) => Err(Refused::Differs(format!("{} is on this machine already and differs: choose a side", named(at)))),
        None => pieces::put_here(places, at.kind, &there, None),
    }
}

/// Settle a row on both sides with the one `side` keeps: this machine's into the home, or the
/// home's over this machine's.
pub fn choose(places: &Places<'_>, at: &HomeRowAt, side: HomeSide, machine: &str) -> Result<(), Refused> {
    let (Some(here), Some(there)) = the_two(places, at) else {
        return Err(Refused::NotThere(format!("{} is not on both sides: add it instead", named(at))));
    };
    if compare::same(at.kind, &here, &there) {
        return Ok(());
    }
    match side {
        HomeSide::Here => published(places, at, &here, Some(&there), format!("Keep {} as {machine} has it", named(at))),
        HomeSide::There => pieces::put_here(places, at.kind, &there, Some(&here)),
    }
}

/// What syncs for `login`'s home, out of the switches kept: **every kind off until it is turned on.**
pub fn kinds(login: &str, switched: &BTreeMap<String, bool>) -> HomeKinds {
    HomeKinds {
        login: login.to_string(),
        kinds: HomeKind::ALL
            .iter()
            .map(|kind| HomeKindSwitch { kind: *kind, on: switched.get(kind.as_wire()).copied().unwrap_or(false) })
            .collect(),
    }
}

/// The kinds switched on.
pub fn on(switched: &BTreeMap<String, bool>) -> Vec<HomeKind> {
    kinds("", switched).kinds.into_iter().filter(|kind| kind.on).map(|kind| kind.kind).collect()
}
