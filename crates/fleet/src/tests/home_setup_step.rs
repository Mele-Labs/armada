//! The sign-in's last setup step, the shared home, read while a sign-in is
//! still setting up: done and named once this machine holds a copy of that
//! login's own home, waiting while it does not. The store's enrolment is held open so the flow
//! can be read mid-setup, and let go once it has been.

use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{HomePlace, HomeRepositories, Secret, SignedInAs, StoreError};
use ipc::{AccountFlow, SetUpState};
use testkit::{FakeHomes, FakeRemote, FakeSignIn};

use crate::signing_in::{Enrolling, SigningIn};
use crate::tests::planted::Ticking;
use crate::tests::signing_in::{begin, quickly, settled, Enrolled};
use crate::tests::tmp::TempDir;

/// A store that enrols only once it is let go.
struct Held {
    open: Mutex<bool>,
    let_go: Condvar,
    store: Enrolled,
}

impl Held {
    fn let_go(&self) {
        *self.open.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.let_go.notify_all();
    }
}

impl Enrolling for Held {
    fn enrol(&self, url: &str, directory: &Path, token: &Secret<String>, machine: &str) -> Result<String, StoreError> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        while !*open {
            open = self.let_go.wait(open).unwrap_or_else(PoisonError::into_inner);
        }
        drop(open);
        self.store.enrol(url, directory, token, machine)
    }

    fn delete_device(&self, url: &str, directory: &Path, device: &str) -> Result<(), StoreError> {
        self.store.delete_device(url, directory, device)
    }
}

/// The home's step and the repository a sign-in names while its store step is working.
fn mid_setup(homes: FakeHomes) -> (SetUpState, String) {
    let dir = TempDir::new();
    let forge = Arc::new(FakeSignIn::new().grants("tok", SignedInAs { id: 583_231, login: "octocat".into() }));
    let store = Arc::new(Held { open: Mutex::new(false), let_go: Condvar::new(), store: Enrolled::new() });
    let signing = SigningIn::new(dir.path(), forge, store.clone(), Arc::new(Ticking::from_nine())).paced(quickly());
    signing.knowing_home(Arc::new(move |login: &str, owners: &[String]| homes.of(login, owners).ok()?.here()));
    signing.start(begin(), &[]).expect("begun");
    let deadline = Instant::now() + Duration::from_secs(20);
    let read = loop {
        let read = signing.accounts(&[]).expect("reads");
        if let AccountFlow::Setup { repository, steps, .. } = read.flow {
            if steps.store == SetUpState::Working {
                break (steps.home, repository);
            }
        }
        assert!(Instant::now() < deadline, "the sign-in never reached its store step");
        std::thread::sleep(Duration::from_millis(2));
    };
    store.let_go();
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
    assert_eq!(mid_setup(FakeHomes::new()), (SetUpState::Waiting, String::new()), "the forge here does not hold the login");
    assert_eq!(
        mid_setup(opened(&FakeRemote::of("hubot"))),
        (SetUpState::Waiting, String::new()),
        "another profile's home is here, and this login's is not"
    );
    assert_eq!(
        mid_setup(opened(&FakeRemote::of("octocat"))),
        (SetUpState::Done, "octocat/armada-home".to_string()),
        "a copy of its own home here: the step is done and names it"
    );
}
