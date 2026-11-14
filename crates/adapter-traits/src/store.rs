//! The central store: one small service every machine signed in to an account
//! reaches, holding claims of every kind (a tracker item, a file, a test, a red
//! main, a pull request) and the registered owner of a pull request.
//! `git show ecfffb2c1ebd:docs/capabilities/merge-line.md`, *Fixing a red across machines*.
//!
//! # A trait of its own, so [`Delivery`](crate::Delivery) keeps being per-forge
//!
//! Delivery is what a forge does with a Job's branch, and its fake and its
//! implementation would both churn for a service that is not the forge. The
//! store is held by the caller that is allowed to reach the network, and by no
//! Drone.
//!
//! # The store never sees a name of a thing
//!
//! A repository, a claimed thing and a pull request are each an opaque hash made
//! on the machine, and a key type refuses anything that is not one. A path
//! separator or a space cannot reach the wire, because the type that would
//! carry it does not exist. **Who holds a claim is the exception**: the person's
//! name, the Mac and the Job's title are sent readable, so another Mac can say who
//! (the owner's ruling of 13 Nov 2026, `.claude/decisions/2026-11-13-claims-store.md`).
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
    /// The thing a claim is on, as a hash made on the machine: for a red main, the
    /// commit main went red at.
    ClaimKey
);
hash_key!(
    /// A pull request, as a hash made on the machine.
    OwnerKey
);

/// What a claim is on. The same [`ClaimKey`] under two kinds is two claims.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClaimKind {
    Item,
    File,
    Test,
    Red,
    PullRequest,
}

impl ClaimKind {
    /// The word the store writes, in a body and in a path.
    pub fn as_wire(self) -> &'static str {
        match self {
            ClaimKind::Item => "item",
            ClaimKind::File => "file",
            ClaimKind::Test => "test",
            ClaimKind::Red => "red",
            ClaimKind::PullRequest => "pull_request",
        }
    }

    pub fn from_wire(word: &str) -> Option<ClaimKind> {
        [ClaimKind::Item, ClaimKind::File, ClaimKind::Test, ClaimKind::Red, ClaimKind::PullRequest]
            .into_iter()
            .find(|kind| kind.as_wire() == word)
    }
}

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

/// The longest a person's name may be on a claim.
const LONGEST_PERSON: usize = 64;
/// The longest a Job's title may be on a claim.
const LONGEST_TITLE: usize = 200;

/// Who holds a claim, readable: the holder, the person's name and the Job's title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claimant {
    pub holder: Holder,
    /// The person, as their forge login. Empty where the Mac knows none.
    pub name: String,
    /// The Job's title. Empty where it has none.
    pub title: String,
}

impl Claimant {
    /// **Made to fit rather than refused**, as a machine's name is: a long title is
    /// not a reason to leave a red unclaimed. Control characters are dropped.
    pub fn new(holder: Holder, name: &str, title: &str) -> Claimant {
        let fit = |text: &str, most: usize| -> String { text.chars().filter(|c| !c.is_control()).take(most).collect() };
        Claimant { holder, name: fit(name, LONGEST_PERSON), title: fit(title, LONGEST_TITLE) }
    }
}

/// What a claim came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claimed {
    /// Nobody held it, and this holder does now.
    Won,
    /// Somebody already did, and this is who.
    Held(Claimant),
}

/// Another holder asked for one of this holder's claims and was told it is held. The store keeps it
/// until the holder's next renewal answers it, so a Job learns who else reached the same file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contest {
    pub repository: RepositoryKey,
    pub kind: ClaimKind,
    pub key: ClaimKey,
    pub by: Claimant,
}

/// What a renewal answered: how many claims the holder still has, `0` for one whose claims
/// lapsed, and the contests on them since its last renewal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Renewed {
    pub claims: u64,
    pub contested: Vec<Contest>,
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
/// A device signed in to a store, and the Mac's name it enrolled as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreDevice {
    pub id: String,
    pub machine: String,
}

pub trait Store {
    /// Take the thing if nobody holds it. **One call, answered atomically by the
    /// store**: of two machines racing, the first wins and the other is told who.
    /// A win renews every claim the holder has.
    fn claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError>;

    /// Who holds the thing, if anyone. A claim whose holder stopped renewing for
    /// 30 minutes is gone.
    fn read_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<Option<Claimant>, StoreError>;

    /// Give a claim back, so another machine may take it. Nothing held is not an
    /// error.
    fn release_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey) -> Result<(), StoreError>;

    /// Take the thing from whoever holds it. **Asked for, never assumed**: the
    /// caller has been told who holds it and a person said to take it over.
    fn replace_claim(&self, repository: &RepositoryKey, kind: ClaimKind, key: &ClaimKey, claimant: &Claimant) -> Result<Claimed, StoreError>;

    /// Delete every claim of one kind under a repository: main went green, and its
    /// reds end.
    fn clear_kind(&self, repository: &RepositoryKey, kind: ClaimKind) -> Result<(), StoreError>;

    /// Renew every claim `holder` has, in one call, and answer how many it still
    /// has: `0` for a holder whose claims lapsed, which takes them again. The
    /// holder's `since` is not sent. The answer also names each other holder told
    /// one of these claims was held since the last renewal, once.
    fn renew(&self, holder: &Holder) -> Result<Renewed, StoreError>;

    /// Register the owner of a pull request.
    fn put_owner(&self, key: &OwnerKey, holder: &Holder) -> Result<(), StoreError>;

    /// The registered owners among `keys`; a key nobody owns is absent.
    fn owners(&self, keys: &[OwnerKey]) -> Result<BTreeMap<OwnerKey, Holder>, StoreError>;

    fn delete_owner(&self, key: &OwnerKey) -> Result<(), StoreError>;

    /// Sign a device out of the account, this machine's own or another's: the
    /// store allows any device of an account to, so a lost machine can be
    /// removed. A device already gone is not an error.
    fn delete_device(&self, device: &str) -> Result<(), StoreError>;

    /// Every device signed in to the store this one signs into, oldest first: the account's on a store
    /// of one's own, every member's on a team store.
    fn signed_in_devices(&self) -> Result<Vec<StoreDevice>, StoreError>;

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
        assert!(ClaimKey::from_hash(&"a".repeat(129)).is_none());
    }

    #[test]
    fn a_claimants_name_and_title_are_made_to_fit() {
        let holder = Holder::new(HolderKind::Session, "s1", "mac", 5).expect("a holder");
        let claimant = Claimant::new(holder, &format!("{}\n", "n".repeat(70)), &"t".repeat(250));
        assert_eq!(claimant.name, "n".repeat(64));
        assert_eq!(claimant.title, "t".repeat(200));
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
