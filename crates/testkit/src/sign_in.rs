//! A [`SignIn`] that answers from a script, for the tests that sign in without
//! a forge.
//!
//! # A script of poll answers
//!
//! `FakeSignIn::new().pending(2).grants("token", account)` is a person who types
//! the code on the third poll. After the last scripted answer a poll stays
//! pending, so a sign-in nobody finishes is the default and a test that wants
//! one to end says how.
//!
//! The interval is zero: a test that waits a real second between polls is a
//! test people stop running. The code a person types is the same every time.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard, PoisonError};

use adapter_traits::{DeviceCode, Polled, Secret, SignIn, SignInError, SignedInAs};

/// The code `begin` hands out.
pub const USER_CODE: &str = "WDJB-MJHT";
/// The page it is typed at.
pub const VERIFICATION_URI: &str = "https://forge.test/login/device";
const DEVICE_CODE: &str = "fake-device-code";

enum Step {
    Pending,
    SlowDown,
    Granted(String),
    Expired,
    Denied,
}

#[derive(Default)]
struct State {
    steps: VecDeque<Step>,
    accounts: BTreeMap<String, SignedInAs>,
    begin_refused: Option<String>,
    unreachable: bool,
    begun: Vec<String>,
    polls: usize,
    who_asked: usize,
}

/// A forge that remembers what it was asked and reaches nothing.
#[derive(Default)]
pub struct FakeSignIn {
    state: Mutex<State>,
}

impl FakeSignIn {
    pub fn new() -> FakeSignIn {
        FakeSignIn::default()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The next `times` polls find the code not yet typed.
    pub fn pending(self, times: usize) -> FakeSignIn {
        self.state().steps.extend((0..times).map(|_| Step::Pending));
        self
    }

    /// The next poll is told to wait longer.
    pub fn slow_down(self) -> FakeSignIn {
        self.state().steps.push_back(Step::SlowDown);
        self
    }

    /// The next poll is granted `token`, which `who` then answers with `account`.
    pub fn grants(self, token: &str, account: SignedInAs) -> FakeSignIn {
        {
            let mut state = self.state();
            state.steps.push_back(Step::Granted(token.to_string()));
            state.accounts.insert(token.to_string(), account);
        }
        self
    }

    /// The next poll is granted `token`, which `who` will not vouch for.
    pub fn granting_a_token_nobody_vouches_for(self, token: &str) -> FakeSignIn {
        self.state().steps.push_back(Step::Granted(token.to_string()));
        self
    }

    pub fn expires(self) -> FakeSignIn {
        self.state().steps.push_back(Step::Expired);
        self
    }

    pub fn denies(self) -> FakeSignIn {
        self.state().steps.push_back(Step::Denied);
        self
    }

    /// `begin` is refused with `why`, as a forge does an app with the flow off.
    pub fn refusing_to_begin(self, why: &str) -> FakeSignIn {
        self.state().begin_refused = Some(why.to_string());
        self
    }

    /// Every call fails as unreachable while set.
    pub fn set_unreachable(&self, unreachable: bool) {
        self.state().unreachable = unreachable;
    }

    /// The client id of each `begin`, in order.
    pub fn begun(&self) -> Vec<String> {
        self.state().begun.clone()
    }

    /// How many polls were made.
    pub fn polls(&self) -> usize {
        self.state().polls
    }

    /// How many times `who` was asked.
    pub fn who_asked(&self) -> usize {
        self.state().who_asked
    }
}

fn down() -> SignInError {
    SignInError::Unreachable("the fake forge is switched off".to_string())
}

impl SignIn for FakeSignIn {
    fn begin(&self, client_id: &str) -> Result<DeviceCode, SignInError> {
        let mut state = self.state();
        state.begun.push(client_id.to_string());
        if state.unreachable {
            return Err(down());
        }
        if let Some(why) = &state.begin_refused {
            return Err(SignInError::Refused(why.clone()));
        }
        Ok(DeviceCode {
            user_code: USER_CODE.to_string(),
            verification_uri: VERIFICATION_URI.to_string(),
            device_code: Secret::new(DEVICE_CODE.to_string()),
            interval_seconds: 0,
            expires_in_seconds: 900,
        })
    }

    fn poll(&self, _client_id: &str, device_code: &Secret<String>) -> Result<Polled, SignInError> {
        let mut state = self.state();
        state.polls += 1;
        if state.unreachable {
            return Err(down());
        }
        if device_code.expose() != DEVICE_CODE {
            return Err(SignInError::Refused("The device code is not valid.".to_string()));
        }
        Ok(match state.steps.pop_front() {
            None | Some(Step::Pending) => Polled::Pending,
            Some(Step::SlowDown) => Polled::SlowDown,
            Some(Step::Granted(token)) => Polled::Granted(Secret::new(token)),
            Some(Step::Expired) => Polled::Expired,
            Some(Step::Denied) => Polled::Denied,
        })
    }

    fn who(&self, token: &Secret<String>) -> Result<SignedInAs, SignInError> {
        let mut state = self.state();
        state.who_asked += 1;
        if state.unreachable {
            return Err(down());
        }
        state
            .accounts
            .get(token.expose())
            .cloned()
            .ok_or_else(|| SignInError::Refused("The forge did not accept the token.".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn octocat() -> SignedInAs {
        SignedInAs { id: 583_231, login: "octocat".to_string() }
    }

    #[test]
    fn a_script_of_two_pendings_then_a_grant_is_a_person_typing_the_code_on_the_third_poll() {
        let forge = FakeSignIn::new().pending(2).grants("tok", octocat());
        let code = forge.begin("client").unwrap();
        assert!(matches!(forge.poll("client", &code.device_code), Ok(Polled::Pending)));
        assert!(matches!(forge.poll("client", &code.device_code), Ok(Polled::Pending)));
        let Ok(Polled::Granted(token)) = forge.poll("client", &code.device_code) else { panic!("not granted") };
        assert_eq!(forge.who(&token).unwrap(), octocat());
        assert_eq!((forge.begun(), forge.polls(), forge.who_asked()), (vec!["client".to_string()], 3, 1));
    }

    #[test]
    fn a_script_that_has_run_out_stays_pending_and_an_unknown_token_is_refused() {
        let forge = FakeSignIn::new();
        let code = forge.begin("client").unwrap();
        assert!(matches!(forge.poll("client", &code.device_code), Ok(Polled::Pending)));
        assert!(forge.who(&Secret::new("nope".to_string())).is_err());
    }

    #[test]
    fn expiry_and_denial_are_answers_and_an_unreachable_forge_is_an_error() {
        let forge = FakeSignIn::new().expires().denies();
        let code = forge.begin("client").unwrap();
        assert!(matches!(forge.poll("client", &code.device_code), Ok(Polled::Expired)));
        assert!(matches!(forge.poll("client", &code.device_code), Ok(Polled::Denied)));
        forge.set_unreachable(true);
        assert!(forge.begin("client").err().unwrap().is_unreachable());
        assert!(forge.poll("client", &code.device_code).err().unwrap().is_unreachable());
    }
}
