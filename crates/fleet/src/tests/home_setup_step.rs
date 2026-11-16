//! The sign-in's last setup step, the shared home, read while a sign-in is
//! still setting up: done and named once this machine holds a copy of that
//! login's own home, waiting while it does not. A sign-in enrols with no store, so its
//! store step reads done as soon as it is shown. The clock is held shut so the flow can be
//! read mid-setup, at the moment the profile is to join, and let go once it has been.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{HomePlace, HomeRepositories, SignedInAs};
use core_model::Timestamp;
use ipc::{AccountFlow, SetUpState};
use testkit::{FakeHomes, FakeRemote, FakeSignIn};

use crate::clock::Clock;
use crate::signing_in::SigningIn;
use crate::tests::planted::Ticking;
use crate::tests::signing_in::{begin, quickly, settled, Enrolled};
use crate::tests::tmp::TempDir;

/// A clock that answers only once it is let go: the profile joins with the time it gives, so a
/// sign-in waits at its last step until then.
struct Held {
    open: Mutex<bool>,
    let_go: Condvar,
    ticking: Ticking,
}

impl Held {
    fn let_go(&self) {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.let_go.notify_all();
    }
}

impl Clock for Held {
    fn now(&self) -> Timestamp {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        while !*open {
            open = self.let_go.wait(open).unwrap_or_else(PoisonError::into_inner);
        }
        drop(open);
        self.ticking.now()
    }
}

/// The steps and the repository a sign-in names while it is setting up and has not yet joined.
fn mid_setup(homes: FakeHomes) -> (SetUpState, SetUpState, String) {
    let dir = TempDir::new();
    let forge = Arc::new(FakeSignIn::new().grants("tok", SignedInAs { id: 583_231, login: "octocat".into() }));
    let clock = Arc::new(Held { open: Mutex::new(false), let_go: Condvar::new(), ticking: Ticking::from_nine() });
    let signing = SigningIn::new(dir.path(), forge, Arc::new(Enrolled::new()), clock.clone()).paced(quickly());
    signing.knowing_home(Arc::new(move |login: &str, owners: &[String]| homes.of(login, owners).ok()?.here()));
    signing.start(begin(), &[]).expect("begun");
    let deadline = Instant::now() + Duration::from_secs(20);
    let read = loop {
        let read = signing.accounts(&[]).expect("reads");
        if let AccountFlow::Setup { repository, steps, .. } = read.flow {
            if steps.identity == SetUpState::Done {
                break (steps.store, steps.home, repository);
            }
        }
        assert!(Instant::now() < deadline, "the sign-in never reached its last step");
        std::thread::sleep(Duration::from_millis(2));
    };
    clock.let_go();
    settled(&signing);
    read
}

#[test]
fn the_home_step_of_a_sign_in_is_done_and_names_the_home_once_this_machine_holds_a_copy_of_that_logins_own() {
    let opened = |remote: &FakeRemote| {
        let homes = FakeHomes::new().holding(remote);
        homes.of(remote.login(), &[]).expect("a login").open(&HomePlace::default_for(remote.login())).expect("opens");
        homes
    };
    assert_eq!(
        mid_setup(FakeHomes::new()),
        (SetUpState::Done, SetUpState::Waiting, String::new()),
        "the forge here does not hold the login, and the store was never asked"
    );
    assert_eq!(
        mid_setup(opened(&FakeRemote::of("hubot"))),
        (SetUpState::Done, SetUpState::Waiting, String::new()),
        "another profile's home is here, and this login's is not"
    );
    assert_eq!(
        mid_setup(opened(&FakeRemote::of("octocat"))),
        (SetUpState::Done, SetUpState::Done, "octocat/armada-home".to_string()),
        "a copy of its own home here: the step is done and names it"
    );
}
