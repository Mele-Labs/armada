//! Fleet's side of the account's four operations: reads the two settings and the
//! machine's name, runs the blocking work off the async workers, and spells a
//! refusal on the wire.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Accounts, Refusal};
use config::settings as keys;
use ipc::{Accounts as AccountsList, SetActiveAccount, SignOut, SignedOut, WireError};

use super::{Begin, Refused, SigningIn};
use crate::daemon::Fleet;

/// Sign-in asked for while the sign-in app or the store address is blank. A 422
/// naming which, so the person knows what to put in Settings.
const SIGN_IN_UNAVAILABLE: &str = "fleet.sign_in_unavailable";
/// The forge would not begin a sign-in: its app has the device flow off, or it
/// did not answer. A 422 carrying the forge's own words.
const SIGN_IN_NOT_BEGUN: &str = "fleet.sign_in_not_begun";
/// An act on an account this machine has not joined. A 422.
const NO_SUCH_ACCOUNT: &str = "fleet.no_such_account";
/// The profiles could not be read or kept. A 500, and nothing was changed that
/// the answer does not say.
const ACCOUNTS_NOT_KEPT: &str = "fleet.accounts_not_kept";

/// The longest a machine's name is kept, as the store takes it.
const LONGEST_NAME: usize = 64;

/// What the other machines call this one: `machine.name`, or the host's name
/// where that is blank. **Made to fit rather than refused**, as a holder's is.
pub(crate) fn machine_name(configured: &str, host: impl FnOnce() -> String) -> String {
    let chosen = if configured.trim().is_empty() { host() } else { configured.to_string() };
    let fitted: String = chosen.trim().chars().filter(|c| !c.is_control()).take(LONGEST_NAME).collect();
    if fitted.is_empty() {
        "this machine".to_string()
    } else {
        fitted
    }
}

fn host_name() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// Run `work` where blocking is allowed. The forge and the store are
    /// blocking clients, which panic if used or dropped on an async worker.
    async fn signing<T: Send + 'static>(
        &self,
        work: impl FnOnce(SigningIn) -> Result<T, Refused> + Send + 'static,
    ) -> Result<T, Refusal> {
        let signing = self.signing_in().clone();
        match tokio::task::spawn_blocking(move || work(signing)).await {
            Ok(done) => done.map_err(|refused| self.refused_account(refused)),
            Err(why) => Err(Refusal::Fault(WireError::raised(
                ACCOUNTS_NOT_KEPT,
                format!("The account could not be read: {why}"),
                self.run_id(),
            ))),
        }
    }

    fn refused_account(&self, refused: Refused) -> Refusal {
        let run = self.run_id();
        match refused {
            Refused::Unavailable(said) => Refusal::Unacceptable(WireError::raised(SIGN_IN_UNAVAILABLE, said, run)),
            Refused::NotBegun(said) => Refusal::Unacceptable(WireError::raised(SIGN_IN_NOT_BEGUN, said, run)),
            Refused::NoSuchAccount(id) => Refusal::Unacceptable(WireError::raised(
                NO_SUCH_ACCOUNT,
                format!("This machine has not joined account {id}."),
                run,
            )),
            Refused::Fault(said) => Refusal::Fault(WireError::raised(ACCOUNTS_NOT_KEPT, said, run)),
        }
    }
}

impl<H, V, W> Accounts for Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    async fn get_accounts(&self) -> Result<AccountsList, Refusal> {
        self.signing(|signing| signing.accounts()).await
    }

    /// **The settings are read here and handed down**, so a change to either is
    /// the next sign-in's and a sign-in under way does not meet it half way.
    async fn start_sign_in(&self) -> Result<AccountsList, Refusal> {
        let now = self.machine_settings().now();
        let begin = Begin {
            client_id: now.get(keys::ACCOUNT_CLIENT_ID),
            store_url: now.get(keys::STORE_URL),
            machine: now.get(keys::MACHINE_NAME),
        };
        self.signing(move |signing| {
            let machine = machine_name(&begin.machine, host_name);
            signing.start(Begin { machine, ..begin })
        })
        .await
    }

    async fn sign_out(&self, sign_out: SignOut) -> Result<SignedOut, Refusal> {
        let store_url: String = self.machine_settings().now().get(keys::STORE_URL);
        self.signing(move |signing| signing.sign_out(sign_out.account_id, &store_url)).await
    }

    async fn set_active_account(&self, set: SetActiveAccount) -> Result<AccountsList, Refusal> {
        self.signing(move |signing| signing.set_active(set.account_id)).await
    }
}
