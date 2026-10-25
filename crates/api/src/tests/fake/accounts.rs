//! The account as the fake answers it: a machine signed out until a sign-in
//! starts, a code that is always the same, and an account it has not joined
//! refused by name. What is polled, enrolled and written is `fleet::signing_in`'s.

use ipc::{AccountFlow, Accounts as AccountsList, BoundHow, BoundRepository, Profile, ProfileHome, SetRepositoryProfile, SignOut, SignedOut};

use super::FakeDaemon;
use crate::{Accounts, Refusal};

/// The fake's machine before anyone signs in.
pub(crate) fn signed_out() -> AccountsList {
    AccountsList { profiles: Vec::new(), unmatched: Vec::new(), flow: AccountFlow::Idle { ended: None } }
}

fn not_joined(account_id: u64) -> Refusal {
    Refusal::Unacceptable(ipc::WireError::raised(
        "fleet.no_such_account",
        format!("this machine has not joined account {account_id}"),
        crate::tests::shapes::run_id(),
    ))
}

impl Accounts for FakeDaemon {
    async fn get_accounts(&self) -> Result<AccountsList, Refusal> {
        Ok(self.accounts.lock().expect("not poisoned").clone())
    }

    async fn start_sign_in(&self) -> Result<AccountsList, Refusal> {
        let mut held = self.accounts.lock().expect("not poisoned");
        held.flow = AccountFlow::Code {
            code: "WDJB-MJHT".to_string(),
            address: "https://forge.test/login/device".to_string(),
            expires_in_seconds: 900,
        };
        Ok(held.clone())
    }

    async fn sign_out(&self, sign_out: SignOut) -> Result<SignedOut, Refusal> {
        let mut held = self.accounts.lock().expect("not poisoned");
        let before = held.profiles.len();
        held.profiles.retain(|one| one.account_id != sign_out.account_id);
        if held.profiles.len() == before {
            return Err(not_joined(sign_out.account_id));
        }
        Ok(SignedOut { accounts: held.clone(), store_row_remains: false, said: None })
    }

    /// The repository moves out of `unmatched` (or out of the profile that had it) into the
    /// profile chosen, or stays unmatched with `none_chosen` set for "None".
    async fn set_repository_profile(&self, set: SetRepositoryProfile) -> Result<AccountsList, Refusal> {
        let mut held = self.accounts.lock().expect("not poisoned");
        if let Some(account_id) = set.account_id {
            if !held.profiles.iter().any(|one| one.account_id == account_id) {
                return Err(not_joined(account_id));
            }
        }
        let mut found = None;
        for one in &mut held.profiles {
            if let Some(at) = one.repositories.iter().position(|bound| bound.root == set.root) {
                found = Some((one.repositories.remove(at).name, Vec::new()));
            }
        }
        if let Some(at) = held.unmatched.iter().position(|loose| loose.root == set.root) {
            let loose = held.unmatched.remove(at);
            found = Some((loose.name, loose.candidates));
        }
        let Some((name, candidates)) = found else {
            return Err(Refusal::Unacceptable(ipc::WireError::raised(
                "fleet.no_such_repository",
                format!("Fleet does not serve {}", set.root),
                crate::tests::shapes::run_id(),
            )));
        };
        match set.account_id {
            Some(account_id) => {
                let bound = BoundRepository { root: set.root, name, how: BoundHow::Chosen };
                if let Some(one) = held.profiles.iter_mut().find(|one| one.account_id == account_id) {
                    one.repositories.push(bound);
                }
            }
            None => held.unmatched.push(ipc::UnmatchedRepository { root: set.root, name, candidates, none_chosen: true }),
        }
        Ok(held.clone())
    }
}

/// A profile the fake can be planted with.
pub fn profile(account_id: u64, login: &str, chosen: bool) -> Profile {
    Profile {
        account_id,
        login: login.to_string(),
        orgs: Vec::new(),
        chosen,
        home: ProfileHome { account: login.to_string(), repository: "armada-home".to_string(), made: false },
        repositories: Vec::new(),
    }
}
