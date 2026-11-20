//! Fleet's side of the account's operations: reads the two settings and the
//! machine's name, runs the blocking work off the async workers, keeps a
//! registered sign-in app's client id, and spells a refusal on the wire.

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::{Accounts, Refusal};
use config::settings as keys;
use ipc::{Accounts as AccountsList, SaveSettings, SetRepositoryProfile, SettingValue, SignInAppRegistration, SignOut, SignedOut, WireError};

use super::{Begin, Refused, SigningIn};
use crate::daemon::Fleet;
use crate::repositories::NO_SUCH_REPOSITORY;
use crate::repository_profiles::Repo;

/// Sign-in asked for while the sign-in app or the store address is blank. A 422
/// naming which, so the person knows what to put in Settings.
const SIGN_IN_UNAVAILABLE: &str = "fleet.sign_in_unavailable";
/// The forge would not begin a sign-in: its app has the device flow off, or it
/// did not answer. A 422 carrying the forge's own words.
const SIGN_IN_NOT_BEGUN: &str = "fleet.sign_in_not_begun";
/// A state for registering the sign-in app that Fleet did not issue, was used or
/// lapsed. A 422, said to the person in the browser that carried it.
const APP_STATE_UNKNOWN: &str = "fleet.sign_in_app_state_unknown";
/// The forge would not convert the code into an app, or could not be reached. A
/// 422 in its words, and the sign-in app is left as it was.
const APP_NOT_REGISTERED: &str = "fleet.sign_in_app_not_registered";
/// Where the browser opens the form and comes back, on Fleet's own port.
const APP_FORM_PATH: &str = "/accounts/sign_in_app/form";
const APP_BACK_PATH: &str = "/accounts/sign_in_app/registered";
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

/// [`machine_name`] for the host this runs on. **Starts a process**, so off the async workers.
pub(crate) fn this_machine(configured: &str) -> String {
    machine_name(configured, host_name)
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
    /// Run `work` where blocking is allowed, with the repositories Fleet serves.
    /// The forge and the store are blocking clients, which panic if used or
    /// dropped on an async worker.
    async fn signing<T: Send + 'static>(
        &self,
        work: impl FnOnce(SigningIn, Vec<Repo>) -> Result<T, Refused> + Send + 'static,
    ) -> Result<T, Refusal> {
        let signing = self.signing_in().clone();
        let roots: Vec<String> = self.repositories().every().iter().map(|one| one.root().to_string()).collect();
        match tokio::task::spawn_blocking(move || {
            let repositories = roots.iter().map(|root| Repo::new(root)).collect();
            work(signing, repositories)
        })
        .await
        {
            Ok(done) => done.map_err(|refused| self.refused_account(refused)),
            Err(why) => Err(Refusal::Fault(WireError::raised(
                ACCOUNTS_NOT_KEPT,
                format!("The account could not be read: {why}"),
                self.run_id(),
            ))),
        }
    }

    /// Where each profile's home is made, from what this machine recorded for it, in place of the
    /// default a profile with no copy here is read with.
    async fn homed(&self, mut accounts: AccountsList) -> Result<AccountsList, Refusal> {
        let store = self.store().lock().await;
        for profile in accounts.profiles.iter_mut().filter(|profile| !profile.home.made) {
            let kept = store.home_repository(&profile.login).map_err(|why| {
                Refusal::Fault(WireError::raised(ACCOUNTS_NOT_KEPT, format!("Where the shared home is made could not be read: {why}"), self.run_id()))
            })?;
            if let Some(kept) = kept {
                profile.home.account = kept.owner;
                profile.home.repository = kept.name;
            }
        }
        Ok(accounts)
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
            Refused::NoSuchRepository(root) => Refusal::Unacceptable(WireError::raised(
                NO_SUCH_REPOSITORY,
                format!("Fleet does not serve {root}."),
                run,
            )),
            Refused::Fault(said) => Refusal::Fault(WireError::raised(ACCOUNTS_NOT_KEPT, said, run)),
            Refused::AppStateUnknown => Refusal::Unacceptable(WireError::raised(
                APP_STATE_UNKNOWN,
                "This page has expired or was already used. Press Register in Armada's Settings again.",
                run,
            )),
            Refused::AppNotRegistered(said) => Refusal::Unacceptable(WireError::raised(APP_NOT_REGISTERED, said, run)),
        }
    }

    /// An address on Fleet's own listener, which the person's browser reaches.
    fn on_this_listener(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.host().port)
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
        let accounts = self.signing(|signing, repositories| signing.accounts(&repositories)).await?;
        self.homed(accounts).await
    }

    /// **The settings are read here and handed down**, so a change to either is
    /// the next sign-in's and a sign-in under way does not meet it half way.
    async fn start_sign_in(&self) -> Result<AccountsList, Refusal> {
        let begin = Begin { client_id: self.machine_settings().now().get(keys::ACCOUNT_CLIENT_ID) };
        let accounts = self.signing(move |signing, repositories| signing.start(begin, &repositories)).await?;
        self.homed(accounts).await
    }

    async fn register_sign_in_app(&self) -> Result<SignInAppRegistration, Refusal> {
        let state = self.signing_in().issue_app_state();
        Ok(SignInAppRegistration { url: format!("{}?state={state}", self.on_this_listener(APP_FORM_PATH)) })
    }

    async fn get_sign_in_app_form(&self, state: String) -> Result<String, Refusal> {
        self.signing_in().app_form(&state, &self.on_this_listener(APP_BACK_PATH)).map_err(|refused| self.refused_account(refused))
    }

    /// **The client id is written as a save of settings would**, so the list is
    /// published and a client reading `get_settings` sees it. Nothing else the
    /// forge answered is kept.
    async fn sign_in_app_registered(&self, code: String, state: String) -> Result<String, Refusal> {
        let signing = self.signing_in().clone();
        let app = tokio::task::spawn_blocking(move || signing.register_app(&code, &state))
            .await
            .map_err(|why| Refusal::Fault(WireError::raised(ACCOUNTS_NOT_KEPT, format!("The app could not be registered: {why}"), self.run_id())))?
            .map_err(|refused| self.refused_account(refused))?;
        let client_id = SettingValue::Text(app.client_id);
        let save = SaveSettings { changes: [(keys::Key::name(keys::ACCOUNT_CLIENT_ID).to_string(), Some(client_id))].into() };
        self.save_settings_now(save).await?;
        Ok(app.settings_page)
    }

    async fn sign_out(&self, sign_out: SignOut) -> Result<SignedOut, Refusal> {
        let mut signed_out = self.signing(move |signing, repositories| signing.sign_out(sign_out.account_id, &repositories)).await?;
        signed_out.accounts = self.homed(signed_out.accounts).await?;
        Ok(signed_out)
    }

    async fn set_repository_profile(&self, set: SetRepositoryProfile) -> Result<AccountsList, Refusal> {
        let accounts = self
            .signing(move |signing, repositories| signing.set_repository_profile(&set.root, set.account_id, &repositories))
            .await?;
        self.homed(accounts).await
    }
}
