//! The account this machine is signed in to: its profiles, and the sign-in that
//! adds one. Its own surface, for [`Settings`](super::Settings)' reason: whole on
//! its own, and a handler taking only this cannot reach for a Job.
//! `docs/concepts/account.md`.

use std::future::Future;

use ipc::{Accounts as AccountsList, SetRepositoryProfile, SignInAppRegistration, SignOut, SignedOut};

use crate::daemon::Refusal;

pub trait Accounts: Send + Sync + 'static {
    /// `get_accounts` — the profiles joined, the repositories each is bound to,
    /// and the sign-in in flight. The only `Refusal` is a fault.
    fn get_accounts(&self) -> impl Future<Output = Result<AccountsList, Refusal>> + Send;

    /// `start_sign_in` — begin the device flow and answer with the code to type.
    /// Fleet then polls by itself. [`Refusal::Unacceptable`] while sign-in is
    /// not set up on this machine, and where the forge would not begin one.
    fn start_sign_in(&self) -> impl Future<Output = Result<AccountsList, Refusal>> + Send;

    /// `register_sign_in_app` — a fresh one-time state, and the page on this
    /// listener that carries the app's manifest to the forge with it.
    fn register_sign_in_app(&self) -> impl Future<Output = Result<SignInAppRegistration, Refusal>> + Send;

    /// `get_sign_in_app_form` — the whole page for a live `state`, which it leaves
    /// live. [`Refusal::Unacceptable`] for any other state.
    fn get_sign_in_app_form(&self, state: String) -> impl Future<Output = Result<String, Refusal>> + Send;

    /// `sign_in_app_registered` — use `state` up, convert `code` and keep the app's
    /// client id, answering the forge's page for its settings.
    /// [`Refusal::Unacceptable`] for a state not live, converting nothing, and for
    /// a code the forge would not convert, keeping nothing.
    fn sign_in_app_registered(&self, code: String, state: String) -> impl Future<Output = Result<String, Refusal>> + Send;

    /// `sign_out` — sign a profile out of the store and delete it from this
    /// machine. **The local profile goes even where the store cannot be asked.**
    /// [`Refusal::Unacceptable`] for an account this machine has not joined.
    fn sign_out(&self, sign_out: SignOut) -> impl Future<Output = Result<SignedOut, Refusal>> + Send;

    /// `set_repository_profile` — which profile a repository claims and syncs as,
    /// or none. [`Refusal::Unacceptable`] for an account this machine has not
    /// joined and for a repository Fleet does not serve.
    fn set_repository_profile(
        &self,
        set: SetRepositoryProfile,
    ) -> impl Future<Output = Result<AccountsList, Refusal>> + Send;
}
