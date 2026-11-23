//! The walk's and the tests' accounts: what `GET /accounts` reads and what signing in, signing out and choosing a
//! repository's login change. Two logins are signed in, as the picked take of the Account page draws them; a walk
//! sets a sign-in that waits on its code or one that ended. Fleet's sentences are copied from what the routes answer
//! with (`crates/ipc/operations/start_sign_in.toml`, `sign_out.toml`, `set_repository_profile.toml`).

use ipc::{AccountFlow, Accounts, BoundHow, BoundRepository, KitLink, Profile, ProfileHome, SignInEnded, SignInOutcome, SignedOut};

use super::wire::Refused;

/// The forge's id of his own login, and of his work one.
pub const PERSONAL: u64 = 5_201_001;
pub const WORK: u64 = 7_340_212;

/// The code a sign-in shows, and where it is typed.
pub const CODE: &str = "WDJB-MJHT";
pub const ADDRESS: &str = "https://github.com/login/device";

/// How long a new code has, as the forge gives it.
const CODE_SECONDS: u64 = 899;

/// What Fleet says of a sign-in whose code ran out.
pub const EXPIRED_SAID: &str = "The code ran out before it was typed. Start again for a new one.";

/// One call the page made, oldest first, for a test to read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountCall {
    StartSignIn,
    SignOut(u64),
    SetRepository { root: String, account_id: Option<u64> },
}

pub struct MockAccounts {
    pub accounts: Accounts,
    /// Kit over the Mac link, as `get_kit_link` answers it for the Account page's Kit row: Kit's own mock link.
    pub kit: KitLink,
    pub calls: Vec<AccountCall>,
    /// A sentence the next start is refused with, as Fleet refuses one the forge would not begin.
    pub start_refusal: Option<String>,
}

fn profile(account_id: u64, login: &str, orgs: &[&str], chosen: bool) -> Profile {
    Profile {
        account_id,
        login: login.into(),
        orgs: orgs.iter().map(|one| one.to_string()).collect(),
        chosen,
        home: ProfileHome { account: login.into(), repository: "armada-home".into(), made: false },
        repositories: Vec::new(),
    }
}

impl MockAccounts {
    /// His own login and his work one, signed in on this Mac, and no sign-in in flight. The armada repository claims as
    /// the work login over its project's.
    pub fn new() -> MockAccounts {
        let mut work = profile(WORK, "nmele-work", &["acme"], true);
        work.repositories.push(BoundRepository { root: "/Users/user/Development/armada".into(), name: "armada".into(), how: BoundHow::Chosen });
        let accounts = Accounts { profiles: vec![profile(PERSONAL, "nickmele", &[], false), work], unmatched: Vec::new(), flow: AccountFlow::Idle { ended: None } };
        MockAccounts { accounts, kit: crate::kit::mock::MockKit::new().link, calls: Vec::new(), start_refusal: None }
    }

    /// A sign-in waits on its code.
    pub fn waiting(&mut self) {
        self.accounts.flow = AccountFlow::Code { code: CODE.into(), address: ADDRESS.into(), expires_in_seconds: CODE_SECONDS };
    }

    /// The last sign-in ended with its code run out.
    pub fn ended(&mut self) {
        self.accounts.flow = AccountFlow::Idle { ended: Some(SignInEnded { outcome: SignInOutcome::Expired, said: Some(EXPIRED_SAID.into()) }) };
    }

    pub fn start_sign_in(&mut self) -> Result<Accounts, Refused> {
        self.calls.push(AccountCall::StartSignIn);
        if let Some(said) = self.start_refusal.clone() {
            return Err(Refused { key: None, said, code: Some("fleet.sign_in_not_begun".into()) });
        }
        self.waiting();
        Ok(self.accounts.clone())
    }

    pub fn sign_out(&mut self, account_id: u64) -> Result<SignedOut, Refused> {
        self.calls.push(AccountCall::SignOut(account_id));
        if !self.accounts.profiles.iter().any(|one| one.account_id == account_id) {
            return Err(Refused { key: None, said: "This Mac has not joined that account.".into(), code: Some("fleet.no_such_account".into()) });
        }
        self.accounts.profiles.retain(|one| one.account_id != account_id);
        Ok(SignedOut { accounts: self.accounts.clone(), store_row_remains: false, said: None })
    }

    /// A repository's login over its project's, by its root. None claims nothing, and is kept by Fleet without a
    /// profile to list it under.
    pub fn set_repository(&mut self, root: &str, account_id: Option<u64>) -> Result<Accounts, Refused> {
        self.calls.push(AccountCall::SetRepository { root: root.into(), account_id });
        if account_id.is_some_and(|id| !self.accounts.profiles.iter().any(|one| one.account_id == id)) {
            return Err(Refused { key: None, said: "This Mac has not joined that account.".into(), code: Some("fleet.no_such_account".into()) });
        }
        let name = root.rsplit('/').next().unwrap_or(root).to_string();
        for one in &mut self.accounts.profiles {
            one.repositories.retain(|bound| bound.root != root);
            if Some(one.account_id) == account_id {
                one.repositories.push(BoundRepository { root: root.into(), name: name.clone(), how: BoundHow::Chosen });
            }
        }
        Ok(self.accounts.clone())
    }
}
