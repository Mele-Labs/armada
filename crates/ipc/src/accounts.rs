//! The account a machine is signed in to, on the wire: the profiles it has
//! joined, which repositories each is bound to, the sign-in in flight, and the
//! acts on them. `docs/concepts/account.md`.
//!
//! **Shaped for what Bridge's Account draws**: a profile, a flow that is idle,
//! showing a code, or setting up row by row. A profile is keyed by the forge's
//! numeric account id and also carries its login, which is what a chip draws.
//! **A repository is bound to one profile, not the machine to one**: there is no
//! active profile.

use serde::{Deserialize, Serialize};

/// One profile this machine has joined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    /// The forge's numeric id for the account, which names the profile in every act.
    pub account_id: u64,
    pub login: String,
    /// Added beside the machine's own on purpose, so what it syncs is a choice.
    pub chosen: bool,
    /// The repositories whose claims and syncing go to this profile's store view.
    pub repositories: Vec<BoundRepository>,
}

/// How a repository came to be bound to its profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundHow {
    /// Its owner is the profile's login or one of its organisations, and no other profile's.
    Matched,
    /// A person chose it, which wins over a match.
    Chosen,
}

/// A repository bound to a profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundRepository {
    /// The repository's root, which names it in `set_repository_profile`.
    pub root: String,
    /// The folder's own name.
    pub name: String,
    pub how: BoundHow,
}

/// A repository bound to no profile, so it claims and syncs nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmatchedRepository {
    pub root: String,
    pub name: String,
    /// The profiles its owner matched. Empty where none did; two or more where
    /// the owner is an organisation they share.
    pub candidates: Vec<u64>,
    /// A person chose "None" for it, so it stays unbound and is not waiting on a choice.
    pub none_chosen: bool,
}

/// How far one row of what signing in sets up has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetUpState {
    Waiting,
    Working,
    Done,
}

/// What signing in sets up, row by row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetUpSteps {
    pub identity: SetUpState,
    pub store: SetUpState,
    /// **Waits until this machine holds a copy of the shared home**, which Sync makes.
    pub home: SetUpState,
}

/// How a sign-in that is no longer in flight ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignInOutcome {
    SignedIn,
    /// The person turned it down at the forge.
    Denied,
    /// The code ran out before it was typed.
    Expired,
    /// The forge or the store did not answer.
    Unreachable,
    /// The forge or the store answered no.
    Refused,
}

/// The last sign-in's end, held until the next one starts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignInEnded {
    pub outcome: SignInOutcome,
    /// What went wrong, in a sentence. Absent for a sign-in that succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
}

/// The sign-in in flight, tagged on `phase`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum AccountFlow {
    /// Nothing in flight. `ended` is how the last one finished, if one has.
    Idle {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ended: Option<SignInEnded>,
    },
    /// The code, shown until it is typed at `address`.
    Code {
        /// The short code to type, dashes and all.
        code: String,
        /// The forge's page where it is typed.
        address: String,
        /// How long the code has left.
        expires_in_seconds: u64,
    },
    /// The code was typed and Fleet is setting the account up.
    Setup {
        login: String,
        /// The shared home's repository. **Empty until a shared home exists.**
        repository: String,
        steps: SetUpSteps,
    },
}

/// `get_accounts`, and what every act on the account answers with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accounts {
    /// In the order they were joined. Empty is signed out.
    pub profiles: Vec<Profile>,
    /// Every served repository that is bound to no profile.
    pub unmatched: Vec<UnmatchedRepository>,
    pub flow: AccountFlow,
}

/// `sign_out`: which profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignOut {
    pub account_id: u64,
}

/// What `sign_out` answers: the profiles left, and whether the store still
/// holds this machine's device.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedOut {
    pub accounts: Accounts,
    /// The profile is gone from this machine either way. `true` where the store
    /// could not be asked to forget the device, so its row remains.
    pub store_row_remains: bool,
    /// Why the store row remains, in a sentence. Absent when it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
}

/// `set_repository_profile`: which profile a repository is bound to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetRepositoryProfile {
    /// A served repository's root.
    pub root: String,
    /// `null` is "None": the repository claims and syncs nothing. **The key must be
    /// there**, so a body that forgot it is a mistake and not a choice of none.
    #[serde(deserialize_with = "stated")]
    pub account_id: Option<u64>,
}

fn stated<'de, D>(input: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<u64>::deserialize(input)
}
