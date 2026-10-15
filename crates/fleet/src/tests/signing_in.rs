//! Signing in, end to end over a scripted forge and a fake store and a real
//! machine directory: the profile a sign-in leaves, what is on disk after it,
//! and what each way of failing leaves behind.
//!
//! **Waits for the thread, never for time.** The poll runs on a thread of its
//! own and a test reads what it left once it has said it is done, so no case
//! sleeps for a length it tuned on a quiet machine.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{Secret, SignedInAs, StoreError};
use ipc::{AccountFlow, Accounts, SignInOutcome};
use testkit::{FakeSignIn, FakeStore};

use crate::profiles;
use crate::signing_in::{Begin, Enrolling, Pacing, Refused, SigningIn};
use crate::tests::planted::Ticking;
use crate::tests::tmp::TempDir;

/// A store that enrols a key the way the real one does and holds its devices
/// in memory: the device id is the account's, a dot and 32 hex characters.
pub(crate) struct Enrolled {
    store: FakeStore,
    minted: AtomicU64,
    refusing: AtomicBool,
    tokens: Mutex<Vec<String>>,
    machines: Mutex<Vec<String>>,
}

impl Enrolled {
    pub(crate) fn new() -> Enrolled {
        Enrolled {
            store: FakeStore::new(),
            minted: AtomicU64::new(0),
            refusing: AtomicBool::new(false),
            tokens: Mutex::new(Vec::new()),
            machines: Mutex::new(Vec::new()),
        }
    }

    /// The machine name each enrolment carried.
    pub(crate) fn machines(&self) -> Vec<String> {
        self.machines.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn tokens(&self) -> Vec<String> {
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl Enrolling for Enrolled {
    fn enrol(&self, _url: &str, directory: &Path, token: &Secret<String>, machine: &str) -> Result<String, StoreError> {
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner).push(token.expose().clone());
        self.machines.lock().unwrap_or_else(PoisonError::into_inner).push(machine.to_string());
        if self.refusing.load(Ordering::SeqCst) {
            return Err(StoreError::Refused("the forge refused the token".to_string()));
        }
        adapters::DeviceKey::open(directory).map_err(StoreError::Refused)?;
        let account = directory.file_name().and_then(|name| name.to_str()).unwrap_or("0");
        let device = format!("{account}.{:032x}", self.minted.fetch_add(1, Ordering::SeqCst) + 1);
        adapters::remember_device(directory, &device).map_err(StoreError::Refused)?;
        self.store.enrol(&device);
        Ok(device)
    }

    fn delete_device(&self, _url: &str, _directory: &Path, device: &str) -> Result<(), StoreError> {
        use adapter_traits::Store;
        self.store.delete_device(device)
    }
}

struct Rig {
    home: TempDir,
    forge: Arc<FakeSignIn>,
    store: Arc<Enrolled>,
    signing: SigningIn,
}

pub(crate) fn quickly() -> Pacing {
    Pacing { floor: Duration::from_millis(1), slow_down: Duration::from_millis(1) }
}

fn rig(forge: FakeSignIn) -> Rig {
    let home = TempDir::new();
    let forge = Arc::new(forge);
    let store = Arc::new(Enrolled::new());
    let signing = SigningIn::new(home.path(), forge.clone(), store.clone(), Arc::new(Ticking::from_nine())).paced(quickly());
    Rig { home, forge, store, signing }
}

pub(crate) fn begin() -> Begin {
    Begin { client_id: "Iv1.abc".into(), store_url: "https://store.test".into(), machine: "mac-a".into() }
}

fn octocat() -> SignedInAs {
    SignedInAs { id: 583_231, login: "octocat".into() }
}

fn hubot() -> SignedInAs {
    SignedInAs { id: 9_919, login: "hubot".into() }
}

/// The sign-in once its thread has finished: the flow is idle again.
pub(crate) fn settled(signing: &SigningIn) -> Accounts {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let read = signing.accounts(&[]).expect("reads");
        if matches!(read.flow, AccountFlow::Idle { .. }) {
            return read;
        }
        assert!(Instant::now() < deadline, "the sign-in never finished: {read:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn ended(read: &Accounts) -> SignInOutcome {
    match &read.flow {
        AccountFlow::Idle { ended: Some(ended) } => ended.outcome,
        other => panic!("not ended: {other:?}"),
    }
}

pub(crate) fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return found };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found
}

pub(crate) fn holds(dir: &Path, needle: &str) -> bool {
    files_under(dir)
        .iter()
        .any(|file| std::fs::read(file).is_ok_and(|bytes| bytes.windows(needle.len()).any(|window| window == needle.as_bytes())))
}

fn the_key_of(rig: &Rig, account: u64) -> String {
    std::fs::read_to_string(rig.home.path().join(profiles::DIRECTORY).join(account.to_string()).join(adapters::KEY_FILE))
        .expect("a key")
}

fn the_device_of(rig: &Rig, account: u64) -> String {
    std::fs::read_to_string(rig.home.path().join(profiles::DIRECTORY).join(account.to_string()).join(adapters::ID_FILE))
        .expect("a device id")
}

#[test]
fn a_full_sign_in_leaves_one_profile_with_a_key_and_a_device_and_no_token_on_disk() {
    let rig = rig(FakeSignIn::new().pending(2).grants("tok-secret-1", octocat()));
    let started = rig.signing.start(begin(), &[]).expect("begun");
    match started.flow {
        AccountFlow::Code { code, address, expires_in_seconds } => {
            assert_eq!(code, testkit::USER_CODE);
            assert_eq!(address, testkit::VERIFICATION_URI);
            assert!((1..=900).contains(&expires_in_seconds), "{expires_in_seconds}");
        }
        other => panic!("{other:?}"),
    }

    let read = settled(&rig.signing);
    assert_eq!(ended(&read), SignInOutcome::SignedIn);
    assert_eq!(read.profiles.len(), 1);
    let profile = &read.profiles[0];
    assert_eq!((profile.account_id, profile.login.as_str(), profile.chosen), (583_231, "octocat", false));

    assert_eq!(rig.forge.begun(), ["Iv1.abc"]);
    assert_eq!((rig.forge.polls(), rig.forge.who_asked()), (3, 1), "pending twice, then granted, then who");
    let device = the_device_of(&rig, 583_231);
    assert!(device.starts_with("583231."), "{device}");
    assert!(!the_key_of(&rig, 583_231).is_empty());
    assert_eq!(rig.store.store.devices(), [device]);
    assert_eq!(rig.store.tokens(), ["tok-secret-1"], "the store was given the token once");

    for secret in ["tok-secret-1", "fake-device-code"] {
        assert!(!holds(rig.home.path(), secret), "{secret} is on disk");
    }
}

#[test]
fn the_profile_directory_and_its_key_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let rig = rig(FakeSignIn::new().grants("t", octocat()));
    rig.signing.start(begin(), &[]).expect("begun");
    settled(&rig.signing);
    let directory = rig.home.path().join(profiles::DIRECTORY).join("583231");
    let mode = |path: &Path| std::fs::metadata(path).expect("there").permissions().mode() & 0o777;
    assert_eq!(mode(&directory), 0o700);
    assert_eq!(mode(&directory.join(adapters::KEY_FILE)), 0o600);
}

#[test]
fn a_second_account_adds_a_second_profile_and_the_first_keeps_its_own_key_and_device() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let sign_in = |forge: FakeSignIn| {
        let forge = Arc::new(forge);
        SigningIn::new(home.path(), forge, store.clone(), Arc::new(Ticking::from_nine())).paced(quickly())
    };

    let first = sign_in(FakeSignIn::new().grants("tok-a", octocat()));
    first.start(begin(), &[]).expect("begun");
    settled(&first);
    let first_dir = home.path().join(profiles::DIRECTORY).join("583231");
    let first_key = std::fs::read_to_string(first_dir.join(adapters::KEY_FILE)).unwrap();
    let first_device = std::fs::read_to_string(first_dir.join(adapters::ID_FILE)).unwrap();

    let second = sign_in(FakeSignIn::new().grants("tok-b", hubot()));
    second.start(begin(), &[]).expect("begun");
    let read = settled(&second);

    assert_eq!(read.profiles.iter().map(|p| p.login.as_str()).collect::<Vec<_>>(), ["octocat", "hubot"]);
    assert_eq!(read.profiles.iter().map(|p| p.chosen).collect::<Vec<_>>(), [false, true]);
    let second_dir = home.path().join(profiles::DIRECTORY).join("9919");
    assert_ne!(std::fs::read_to_string(second_dir.join(adapters::KEY_FILE)).unwrap(), first_key, "profiles never share a key");
    assert_eq!(std::fs::read_to_string(first_dir.join(adapters::KEY_FILE)).unwrap(), first_key);
    assert_eq!(std::fs::read_to_string(first_dir.join(adapters::ID_FILE)).unwrap(), first_device);
    assert_eq!(store.store.devices().len(), 2);
    for secret in ["tok-a", "tok-b"] {
        assert!(!holds(home.path(), secret), "{secret} is on disk");
    }
}

#[test]
fn signing_in_again_as_the_same_account_is_one_profile_with_a_fresh_device_and_the_old_one_signed_out() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let sign_in = |login: &str| {
        let forge = Arc::new(FakeSignIn::new().grants("tok", SignedInAs { id: 583_231, login: login.into() }));
        SigningIn::new(home.path(), forge, store.clone(), Arc::new(Ticking::from_nine())).paced(quickly())
    };
    let first = sign_in("octocat");
    first.start(begin(), &[]).expect("begun");
    settled(&first);
    let directory = home.path().join(profiles::DIRECTORY).join("583231");
    let key = std::fs::read_to_string(directory.join(adapters::KEY_FILE)).unwrap();
    let old = std::fs::read_to_string(directory.join(adapters::ID_FILE)).unwrap();

    let again = sign_in("octocat-renamed");
    again.start(begin(), &[]).expect("begun");
    let read = settled(&again);

    assert_eq!(read.profiles.len(), 1, "the same account is the same profile");
    assert_eq!(read.profiles[0].login, "octocat-renamed", "the login is brought up to date");
    assert_eq!(std::fs::read_to_string(directory.join(adapters::KEY_FILE)).unwrap(), key, "the key is the profile's own");
    let new = std::fs::read_to_string(directory.join(adapters::ID_FILE)).unwrap();
    assert_ne!(new, old);
    assert_eq!(store.store.devices(), [new], "the old device is signed out of the store");
}

fn joined_both(home: &TempDir, store: &Arc<Enrolled>) -> SigningIn {
    let sign_in = |forge: FakeSignIn| {
        SigningIn::new(home.path(), Arc::new(forge), store.clone(), Arc::new(Ticking::from_nine())).paced(quickly())
    };
    let first = sign_in(FakeSignIn::new().grants("tok-a", octocat()));
    first.start(begin(), &[]).expect("begun");
    settled(&first);
    let second = sign_in(FakeSignIn::new().grants("tok-b", hubot()));
    second.start(begin(), &[]).expect("begun");
    settled(&second);
    second
}

#[test]
fn signing_out_removes_only_that_profile_and_signs_only_its_device_out_of_the_store() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = joined_both(&home, &store);
    let kept_device = std::fs::read_to_string(home.path().join(profiles::DIRECTORY).join("9919").join(adapters::ID_FILE)).unwrap();
    let kept_key = std::fs::read_to_string(home.path().join(profiles::DIRECTORY).join("9919").join(adapters::KEY_FILE)).unwrap();

    let out = signing.sign_out(583_231, "https://store.test", &[]).expect("signed out");

    assert!(!out.store_row_remains && out.said.is_none());
    assert_eq!(out.accounts.profiles.len(), 1);
    assert_eq!(out.accounts.profiles[0].login, "hubot");
    assert!(!home.path().join(profiles::DIRECTORY).join("583231").exists(), "its directory is gone");
    assert_eq!(std::fs::read_to_string(home.path().join(profiles::DIRECTORY).join("9919").join(adapters::KEY_FILE)).unwrap(), kept_key);
    assert_eq!(store.store.devices(), [kept_device]);
}

#[test]
fn signing_out_with_the_store_down_still_removes_the_profile_and_says_its_row_remains() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = joined_both(&home, &store);
    store.store.set_unreachable(true);

    let out = signing.sign_out(9_919, "https://store.test", &[]).expect("signed out");

    assert!(out.store_row_remains);
    assert!(out.said.as_deref().is_some_and(|said| said.contains("could not be reached")), "{:?}", out.said);
    assert_eq!(out.accounts.profiles.len(), 1);
    assert!(!home.path().join(profiles::DIRECTORY).join("9919").exists());
    store.store.set_unreachable(false);
    assert_eq!(store.store.devices().len(), 2, "the store was not changed");
}

#[test]
fn signing_out_with_no_store_address_removes_the_profile_and_says_the_store_was_not_asked() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = joined_both(&home, &store);
    let out = signing.sign_out(9_919, "  ", &[]).expect("signed out");
    assert!(out.store_row_remains);
    assert!(out.said.as_deref().is_some_and(|said| said.contains("store address")), "{:?}", out.said);
}

#[test]
fn signing_out_of_an_account_the_machine_never_joined_is_refused_and_changes_nothing() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = joined_both(&home, &store);
    assert_eq!(signing.sign_out(1, "https://store.test", &[]), Err(Refused::NoSuchAccount(1)));
    assert_eq!(signing.accounts(&[]).unwrap().profiles.len(), 2);
}

#[test]
fn a_denied_or_expired_flow_leaves_no_profile_no_directory_and_no_list() {
    for (forge, outcome) in [
        (FakeSignIn::new().pending(1).denies(), SignInOutcome::Denied),
        (FakeSignIn::new().pending(1).expires(), SignInOutcome::Expired),
    ] {
        let rig = rig(forge);
        rig.signing.start(begin(), &[]).expect("begun");
        let read = settled(&rig.signing);
        assert_eq!(ended(&read), outcome);
        assert!(read.profiles.is_empty());
        assert_eq!(rig.forge.who_asked(), 0, "nobody was granted");
        assert!(rig.store.tokens().is_empty() && rig.store.store.devices().is_empty());
        assert!(files_under(rig.home.path()).is_empty(), "{:?}", files_under(rig.home.path()));
    }
}

#[test]
fn a_store_that_refuses_the_enrolment_leaves_no_profile_and_no_key_behind() {
    let rig = rig(FakeSignIn::new().grants("tok", octocat()));
    rig.store.refusing.store(true, Ordering::SeqCst);
    rig.signing.start(begin(), &[]).expect("begun");
    let read = settled(&rig.signing);
    assert_eq!(ended(&read), SignInOutcome::Refused);
    assert!(read.profiles.is_empty());
    assert!(files_under(rig.home.path()).is_empty(), "{:?}", files_under(rig.home.path()));
    assert!(!holds(rig.home.path(), "tok"));
}

#[test]
fn a_forge_that_will_not_say_who_holds_the_token_leaves_no_profile_and_the_store_is_never_asked() {
    let rig = rig(FakeSignIn::new().granting_a_token_nobody_vouches_for("tok-stray"));
    rig.signing.start(begin(), &[]).expect("begun");
    let read = settled(&rig.signing);
    assert_eq!(ended(&read), SignInOutcome::Refused);
    assert!(read.profiles.is_empty());
    assert!(rig.store.tokens().is_empty(), "a token of nobody's is not passed on");
    assert!(files_under(rig.home.path()).is_empty(), "{:?}", files_under(rig.home.path()));
}

#[test]
fn a_blank_sign_in_app_or_store_address_refuses_in_words_that_name_which_and_asks_the_forge_nothing() {
    let rig = rig(FakeSignIn::new());
    let blank = |client: &str, url: &str| {
        rig.signing.start(Begin { client_id: client.into(), store_url: url.into(), machine: "mac-a".into() }, &[]).unwrap_err()
    };
    let Refused::Unavailable(app) = blank("", "https://store.test") else { panic!("not refused as unavailable") };
    assert_eq!(app, "Sign-in is unavailable: the sign-in app is blank in Settings.");
    let Refused::Unavailable(store) = blank("Iv1.abc", "  ") else { panic!("not refused as unavailable") };
    assert_eq!(store, "Sign-in is unavailable: the store address is blank in Settings.");
    let Refused::Unavailable(both) = blank("", "") else { panic!("not refused as unavailable") };
    assert_eq!(both, "Sign-in is unavailable: the sign-in app and the store address are blank in Settings.");
    assert!(rig.forge.begun().is_empty(), "no call was made");
    assert!(matches!(rig.signing.accounts(&[]).unwrap().flow, AccountFlow::Idle { ended: None }));
}

#[test]
fn a_forge_that_will_not_begin_is_refused_with_its_words_and_leaves_the_flow_idle() {
    let rig = rig(FakeSignIn::new().refusing_to_begin("Device Flow must be explicitly enabled for this App"));
    let Refused::NotBegun(said) = rig.signing.start(begin(), &[]).unwrap_err() else { panic!("not refused as unbegun") };
    assert!(said.contains("Device Flow must be explicitly enabled"), "{said}");
    assert!(matches!(rig.signing.accounts(&[]).unwrap().flow, AccountFlow::Idle { ended: None }));

    rig.forge.set_unreachable(true);
    let Refused::NotBegun(said) = rig.signing.start(begin(), &[]).unwrap_err() else { panic!("not refused as unbegun") };
    assert!(said.contains("could not be reached"), "{said}");
}

#[test]
fn a_forge_that_says_slow_down_is_waited_on_longer_and_the_sign_in_still_finishes() {
    let rig = rig(FakeSignIn::new().pending(1).slow_down().grants("tok", octocat()));
    rig.signing.start(begin(), &[]).expect("begun");
    let read = settled(&rig.signing);
    assert_eq!(ended(&read), SignInOutcome::SignedIn);
    assert_eq!(rig.forge.polls(), 3);
}

#[test]
fn the_list_holds_the_numeric_id_the_login_and_when_it_was_added_and_no_key_or_token() {
    let rig = rig(FakeSignIn::new().grants("tok-secret-1", octocat()));
    rig.signing.start(begin(), &[]).expect("begun");
    settled(&rig.signing);
    let text = std::fs::read_to_string(rig.home.path().join(profiles::FILE)).expect("the list");
    for held in ["\"account_id\":583231", "\"login\":\"octocat\"", "\"added_at\":\"2026-"] {
        assert!(text.contains(held), "{held} is not in {text}");
    }
    for never in ["tok-secret-1", "key", "device"] {
        assert!(!text.contains(never), "{never} is in {text}");
    }
}
