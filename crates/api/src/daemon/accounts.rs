//! The account this machine is signed in to: its profiles, and the sign-in that
//! adds one. Its own surface, for [`Settings`](super::Settings)' reason: whole on
//! its own, and a handler taking only this cannot reach for a Job.
//! `docs/concepts/account.md`.

use std::future::Future;

use ipc::{Accounts as AccountsList, SetRepositoryProfile, SignOut, SignedOut};

use crate::daemon::Refusal;

pub trait Accounts: Send + Sync + 'static {
    /// `get_accounts` — the profiles joined, the repositories each is bound to,
    /// and the sign-in in flight. The only `Refusal` is a fault.
    fn get_accounts(&self) -> impl Future<Output = Result<AccountsList, Refusal>> + Send;

    /// `start_sign_in` — begin the device flow and answer with the code to type.
    /// Fleet then polls by itself. [`Refusal::Unacceptable`] while sign-in is
    /// not set up on this machine, and where the forge would not begin one.
    fn start_sign_in(&self) -> impl Future<Output = Result<AccountsList, Refusal>> + Send;

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
