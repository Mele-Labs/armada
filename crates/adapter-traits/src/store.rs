//! The central store: one small service every machine signed in to an account
//! reaches, holding a claim on a red main and the registered owner of a pull
//! request.
//! `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *Fixing a red across machines*.
//!
//! # A trait of its own, so [`Delivery`](crate::Delivery) keeps being per-forge
//!
//! Delivery is what a forge does with a Job's branch, and its fake and its
//! implementation would both churn for a service that is not the forge. The
//! store is held by the caller that is allowed to reach the network, and by no
//! Drone.
//!
//! # The store never sees a name
//!
//! A repository, a red commit and a pull request are each an opaque hash made
//! on the machine, and a key type refuses anything that is not one. A path
//! separator or a space cannot reach the wire, because the type that would
//! carry it does not exist.
//!
//! # A repository's runs are the one thing a row holds in the clear
//!
//! A repository is still only its hash, but [`RunRow`] carries the command a Job ran and the
//! workspace it ran in. The owner chose this on 19 Oct 2026 so how a repository runs follows the
//! account; `docs/contracts/central-store.md`, *Rows*. A row never holds a name, a token or a file.
//!
//! # Unreachable and refused are different errors
//!
//! A take refused because the store could not be reached is not a take that
//! may go ahead: a fix nobody else can see is the double fix this exists to
//! stop. [`StoreError::is_unreachable`] is the question a caller asks.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The longest a hash may be: a SHA-512 in hex.
const LONGEST_KEY: usize = 128;
/// The shortest, so a name short enough to guess is not taken for a hash. The
/// store refuses the same, `docs/contracts/central-store.md`, *Rows*.
const SHORTEST_KEY: usize = 32;
/// The longest a machine name or a holder's id may be.
const LONGEST_NAME: usize = 64;

macro_rules! hash_key {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// The key for a hash of letters, digits, `_` and `-`, 32 to 128
            /// long, or `None` for anything else.
            pub fn from_hash(hash: &str) -> Option<$name> {
                let sound = (SHORTEST_KEY..=LONGEST_KEY).contains(&hash.len())
                    && hash.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
                sound.then(|| $name(hash.to_string()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

hash_key!(
    /// A repository, as a hash made on the machine.
    RepositoryKey
);
hash_key!(
    /// The commit main went red at, as a hash made on the machine.
    RedKey
);
hash_key!(
    /// A pull request, as a hash made on the machine.
    OwnerKey
);

/// What kind of thing holds a claim or owns a pull request. A Session; the store once also
/// took a Job's, and a row it holds for one is refused when read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderKind {
    Session,
}

impl HolderKind {
    /// The word the store writes.
    pub fn as_wire(self) -> &'static str {
        match self {
            HolderKind::Session => "session",
        }
    }

    pub fn from_wire(word: &str) -> Option<HolderKind> {
        match word {
            "session" => Some(HolderKind::Session),
            _ => None,
        }
    }
}

/// Who holds a claim or owns a pull request. **No title, no Check or test name
/// and no repository name**: a row never holds work, and a holder's title is
/// drawn only by the machine that holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holder {
    pub kind: HolderKind,
    /// The Session's short id.
    pub id: String,
    /// The host's name, or `machine.name` where two hosts share one.
    pub machine: String,
    /// Unix seconds the holder took it, so a stale claim reads as stale.
    pub since: i64,
}

impl Holder {
    /// A holder the store will take, or `None` where the id could not be one.
    ///
    /// **The machine's name is made to fit rather than refused**: a host's name
    /// is not the code's to choose, and a take refused for a long one would be
    /// a red nobody could fix.
    pub fn new(kind: HolderKind, id: &str, machine: &str, since: i64) -> Option<Holder> {
        let id_fits = !id.is_empty()
            && id.len() <= LONGEST_NAME
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
        let machine: String = machine.chars().filter(|c| !c.is_control()).take(LONGEST_NAME).collect();
        (id_fits && since >= 0).then(|| Holder { kind, id: id.to_string(), machine, since })
    }
}

/// What a claim came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claimed {
    /// Nobody held it, and this holder does now.
    Won,
    /// Somebody already did, and this is who.
    Held(Holder),
}

/// Why the store did not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    /// Nothing answered: no network, a timeout, a server that is down. The
    /// request may or may not have arrived.
    Unreachable(String),
    /// Something answered and said no: a bad signature, a device it does not
    /// know, an answer this code cannot read.
    Refused(String),
}

impl StoreError {
    pub fn is_unreachable(&self) -> bool {
        matches!(self, StoreError::Unreachable(_))
    }

    /// A sentence for a person, built beside the value as
    /// [`NotDelivered::said`](crate::NotDelivered::said) is.
    pub fn said(&self) -> String {
        match self {
            StoreError::Unreachable(why) => {
                let mut out = String::from("The store could not be reached: ");
                out.push_str(why);
                out
            }
            StoreError::Refused(why) => {
                let mut out = String::from("The store refused the request: ");
                out.push_str(why);
                out
            }
        }
    }
}

/// One remembered run of a repository, as the store keeps it: what `store::KeptRun` is without the
/// place that has taken it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunRow {
    /// Relative to the repository root; `.` is the root.
    pub workspace: String,
    /// The word the wire spells the kind in: `tests`, `lint`, `build`, `dev_server`, `typecheck`
    /// or `format`.
    pub kind: String,
    pub command: String,
    pub port: Option<u16>,
    pub succeeded: bool,
    /// Milliseconds since the epoch.
    pub last_ran: u64,
    pub source: String,
}

/// The central store, as Fleet uses it.
///
/// **Every call is a network round trip**, so a caller on a hot path reads
/// what it already has and calls this only where an answer must be fresh: a
/// take always meets the store itself.
pub trait Store {
    /// Take the red if nobody holds it. **One call, answered atomically by the
    /// store**: of two machines racing, the first wins and the other is told who.
    fn claim(&self, repository: &RepositoryKey, red: &RedKey, holder: &Holder) -> Result<Claimed, StoreError>;

    /// Who holds the red, if anyone.
    fn read_claim(&self, repository: &RepositoryKey, red: &RedKey) -> Result<Option<Holder>, StoreError>;

    /// Give a claim back, so another machine may take the red. Nothing held is
    /// not an error.
    fn release_claim(&self, repository: &RepositoryKey, red: &RedKey) -> Result<(), StoreError>;

    /// Take the red from whoever holds it. **Asked for, never assumed**: the
    /// caller has been told who holds it and a person said to take it over.
    fn replace_claim(&self, repository: &RepositoryKey, red: &RedKey, holder: &Holder) -> Result<Claimed, StoreError>;

    /// Delete every claim under a repository: main went green.
    fn clear_repository(&self, repository: &RepositoryKey) -> Result<(), StoreError>;

    /// Register the owner of a pull request.
    fn put_owner(&self, key: &OwnerKey, holder: &Holder) -> Result<(), StoreError>;

    /// The registered owners among `keys`; a key nobody owns is absent.
    fn owners(&self, keys: &[OwnerKey]) -> Result<BTreeMap<OwnerKey, Holder>, StoreError>;

    fn delete_owner(&self, key: &OwnerKey) -> Result<(), StoreError>;

    /// Sign a device out of the account, this machine's own or another's: the
    /// store allows any device of an account to, so a lost machine can be
    /// removed. A device already gone is not an error.
    fn delete_device(&self, device: &str) -> Result<(), StoreError>;

    /// Replace every run the store holds for a repository. The caller has read and merged first, so
    /// a fact another machine added stays. An empty list leaves the store holding none.
    fn put_runs(&self, repository: &RepositoryKey, runs: &[RunRow]) -> Result<(), StoreError>;

    /// The runs the store holds for a repository, or `None` where it holds none.
    fn read_runs(&self, repository: &RepositoryKey) -> Result<Option<Vec<RunRow>>, StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_hash_of_the_contracts_length_and_alphabet_is_a_key() {
        assert!(RepositoryKey::from_hash(&"0f".repeat(32)).is_some());
        assert!(RepositoryKey::from_hash(&"Ab_-".repeat(8)).is_some());
        for bad in ["", "abc", &"a".repeat(31), &format!("{}/{}", "a".repeat(20), "b".repeat(20)), &"a b".repeat(12), "../../etc/passwd/../../etc/passwd"] {
            assert!(RepositoryKey::from_hash(bad).is_none(), "{bad:?}");
        }
        assert!(RedKey::from_hash(&"a".repeat(129)).is_none());
    }

    #[test]
    fn a_holder_gets_a_machine_name_that_fits_and_an_id_that_does_too() {
        let long = format!("{}\u{7}", "m".repeat(80));
        let holder = Holder::new(HolderKind::Session, "j-2041", &long, 5).expect("a holder");
        assert_eq!(holder.machine, "m".repeat(64));
        for bad in ["", "j 1", "j/1", &"j".repeat(65)] {
            assert!(Holder::new(HolderKind::Session, bad, "mac", 5).is_none(), "{bad:?}");
        }
        assert!(Holder::new(HolderKind::Session, "s1", "mac", -1).is_none());
    }

    #[test]
    fn unreachable_is_told_from_refused() {
        assert!(StoreError::Unreachable("down".into()).is_unreachable());
        assert!(!StoreError::Refused("bad signature".into()).is_unreachable());
    }
}
