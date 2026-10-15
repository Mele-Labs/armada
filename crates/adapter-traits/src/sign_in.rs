//! Signing in to the forge with the device flow: the forge shows a short code,
//! the person types it at the forge's own page, and Fleet polls until the forge
//! hands over an access token. `docs/concepts/account.md`, *Sign-in*.
//!
//! **No client secret exists in this flow**, so none appears here. The token is
//! a [`Secret`], which has no `Debug`, `Display` or `Serialize`, so it cannot
//! reach a log or a file by being formatted.
//!
//! Unreachable and refused are different errors, as they are for the
//! [`Store`](crate::Store): the first is worth another try, the second is not.

use alloc::string::String;
use alloc::vec::Vec;

use crate::secret::Secret;

/// What the forge hands back when a sign-in begins.
///
/// No `Debug`: the device code is what is exchanged for the token.
pub struct DeviceCode {
    /// The short code the person types, as the forge spells it.
    pub user_code: String,
    /// The forge's page where it is typed.
    pub verification_uri: String,
    /// What Fleet polls with. Never shown to the person.
    pub device_code: Secret<String>,
    /// The least time between two polls.
    pub interval_seconds: u64,
    /// How long until the forge stops taking the code.
    pub expires_in_seconds: u64,
}

/// One poll's answer.
///
/// No `Debug`: a granted poll holds the token.
pub enum Polled {
    /// The person has not typed the code yet.
    Pending,
    /// Asked too soon. The next poll waits longer than the last.
    SlowDown,
    Granted(Secret<String>),
    /// The code ran out before it was typed.
    Expired,
    /// The person turned the sign-in down at the forge.
    Denied,
}

/// The account a token belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedInAs {
    /// The forge's numeric id for the account, which is what Armada keys a
    /// profile by: a login can be renamed and an id cannot.
    pub id: u64,
    pub login: String,
}

/// Why the forge did not answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignInError {
    /// Nothing answered: no network, a timeout, a forge that is down.
    Unreachable(String),
    /// Something answered and said no, or said something this code cannot read.
    Refused(String),
}

impl SignInError {
    pub fn is_unreachable(&self) -> bool {
        matches!(self, SignInError::Unreachable(_))
    }

    /// A sentence for a person.
    pub fn said(&self) -> String {
        let (lead, why) = match self {
            SignInError::Unreachable(why) => ("The forge could not be reached: ", why),
            SignInError::Refused(why) => ("The forge refused the sign-in: ", why),
        };
        let mut out = String::from(lead);
        out.push_str(why);
        out
    }
}

/// The forge's device flow, as Fleet uses it.
///
/// **Every call is a network round trip**, and the implementation is blocking:
/// call it from a thread, not an async worker.
pub trait SignIn {
    /// Start a sign-in for the app registered as `client_id`.
    fn begin(&self, client_id: &str) -> Result<DeviceCode, SignInError>;

    /// Ask whether the code has been typed. Called no sooner than the interval
    /// [`begin`](SignIn::begin) gave, and later after [`Polled::SlowDown`].
    fn poll(&self, client_id: &str, device_code: &Secret<String>) -> Result<Polled, SignInError>;

    /// The account a token was issued to.
    fn who(&self, token: &Secret<String>) -> Result<SignedInAs, SignInError>;

    /// The logins of the organisations the token's account belongs to, lowercase.
    /// **Read once at sign-in, while the token is in hand**, and never again:
    /// what matches a repository to a profile is kept, the token is not. An
    /// error is an organisation list that could not be read, which is not a
    /// failed sign-in.
    fn organisations(&self, token: &Secret<String>) -> Result<Vec<String>, SignInError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_is_told_from_refused() {
        assert!(SignInError::Unreachable("down".into()).is_unreachable());
        assert!(!SignInError::Refused("no".into()).is_unreachable());
    }

    #[test]
    fn what_is_said_names_which_of_the_two_it_was() {
        assert!(SignInError::Unreachable("timed out".into()).said().starts_with("The forge could not be reached"));
        assert!(SignInError::Refused("no".into()).said().starts_with("The forge refused"));
    }
}
