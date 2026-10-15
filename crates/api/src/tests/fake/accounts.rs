//! The account as the fake answers it: a machine signed out until a sign-in
//! starts, a code that is always the same, and an account it has not joined
//! refused by name. What is polled, enrolled and written is `fleet::signing_in`'s.

use ipc::{AccountFlow, Accounts as AccountsList, Profile, SetActiveAccount, SignOut, SignedOut};

use super::FakeDaemon;
use crate::{Accounts, Refusal};

/// The fake's machine before anyone signs in.
pub(crate) fn signed_out() -> AccountsList {
    AccountsList { profiles: Vec::new(), flow: AccountFlow::Idle { ended: None } }
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
        if !held.profiles.iter().any(|one| one.active) {
            if let Some(first) = held.profiles.first_mut() {
                first.active = true;
            }
        }
        Ok(SignedOut { accounts: held.clone(), store_row_remains: false, said: None })
    }

    async fn set_active_account(&self, set: SetActiveAccount) -> Result<AccountsList, Refusal> {
        let mut held = self.accounts.lock().expect("not poisoned");
        if !held.profiles.iter().any(|one| one.account_id == set.account_id) {
            return Err(not_joined(set.account_id));
        }
        for one in &mut held.profiles {
            one.active = one.account_id == set.account_id;
        }
        Ok(held.clone())
    }
}

/// A profile the fake can be planted with.
pub fn profile(account_id: u64, login: &str, active: bool, chosen: bool) -> Profile {
    Profile { account_id, login: login.to_string(), active, chosen }
}
