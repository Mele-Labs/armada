//! Signing in, end to end over a scripted forge and a fake store and a real
//! machine directory: the profile a sign-in leaves, what is on disk after it,
//! and what each way of failing leaves behind. A sign-in enrols with no store, so a profile
//! joins one apart from it, with `join_store`, and those cases are here too.
//!
//! **Waits for the thread, never for time.** The poll runs on a thread of its
//! own and a test reads what it left once it has said it is done, so no case
//! sleeps for a length it tuned on a quiet machine.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use adapter_traits::{Secret, SignedInAs, StoreError};
use core_model::StoreAddress;
use ipc::{AccountFlow, Accounts, SignInOutcome};
use testkit::{FakeSignIn, FakeStore};

use crate::profiles;
use crate::signing_in::{Begin, Enrolling, Pacing, Refused, SigningIn};
use crate::tests::planted::Ticking;
use crate::tests::tmp::TempDir;

/// A store that enrols a key the way the real one does and holds its devices
/// in memory: the device id is the folder's name, a dot and 32 hex characters. It records what
/// it was asked, and `gh` is a login's token unless it is made to refuse.
pub(crate) struct Enrolled {
    store: FakeStore,
    minted: AtomicU64,
    refusing: AtomicBool,
    gh_signed_out: AtomicBool,
    tokens: Mutex<Vec<String>>,
    machines: Mutex<Vec<String>>,
    logins: Mutex<Vec<String>>,
    enrolled_at: Mutex<Vec<String>>,
    deleted_at: Mutex<Vec<(String, String)>>,
}

impl Enrolled {
    pub(crate) fn new() -> Enrolled {
        Enrolled {
            store: FakeStore::new(),
            minted: AtomicU64::new(0),
            refusing: AtomicBool::new(false),
            gh_signed_out: AtomicBool::new(false),
            tokens: Mutex::new(Vec::new()),
            machines: Mutex::new(Vec::new()),
            logins: Mutex::new(Vec::new()),
            enrolled_at: Mutex::new(Vec::new()),
            deleted_at: Mutex::new(Vec::new()),
        }
    }

    /// The tokens a store was given to enrol with.
    fn tokens(&self) -> Vec<String> {
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The logins whose token was asked of `gh`.
    fn logins_asked(&self) -> Vec<String> {
        self.logins.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The stores an enrolment was tried at.
    fn enrolled_at(&self) -> Vec<String> {
        self.enrolled_at.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The stores a device was signed out of, as the store's address and the device.
    fn deleted_at(&self) -> Vec<(String, String)> {
        self.deleted_at.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The token this double answers for `login`.
    fn token_of(login: &str) -> String {
        format!("gh-token-of-{login}")
    }
}

impl Enrolling for Enrolled {
    fn enrol(&self, url: &str, directory: &Path, token: &Secret<String>, machine: &str) -> Result<String, StoreError> {
        self.tokens.lock().unwrap_or_else(PoisonError::into_inner).push(token.expose().clone());
        self.machines.lock().unwrap_or_else(PoisonError::into_inner).push(machine.to_string());
        self.enrolled_at.lock().unwrap_or_else(PoisonError::into_inner).push(url.to_string());
        // The key is made before the store is asked, as the real client does.
        adapters::DeviceKey::open(directory).map_err(StoreError::Refused)?;
        if self.refusing.load(Ordering::SeqCst) {
            return Err(StoreError::Refused("the forge refused the token".to_string()));
        }
        let folder = directory.file_name().and_then(|name| name.to_str()).unwrap_or("0");
        let device = format!("{folder}.{:032x}", self.minted.fetch_add(1, Ordering::SeqCst) + 1);
        adapters::remember_device(directory, &device).map_err(StoreError::Refused)?;
        self.store.enrol(&device);
        Ok(device)
    }

    fn delete_device(&self, url: &str, _directory: &Path, device: &str) -> Result<(), StoreError> {
        use adapter_traits::Store;
        self.deleted_at.lock().unwrap_or_else(PoisonError::into_inner).push((url.to_string(), device.to_string()));
        self.store.delete_device(device)
    }

    fn token_for(&self, login: &str) -> Result<Secret<String>, String> {
        self.logins.lock().unwrap_or_else(PoisonError::into_inner).push(login.to_string());
        if self.gh_signed_out.load(Ordering::SeqCst) {
            return Err(format!("gh is not signed in as {login}."));
        }
        Ok(Secret::new(Enrolled::token_of(login)))
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
    Begin { client_id: "Iv1.abc".into() }
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

fn profile_dir(home: &TempDir, account: u64) -> PathBuf {
    home.path().join(profiles::DIRECTORY).join(account.to_string())
}

fn store_a() -> StoreAddress {
    StoreAddress::new("https://store-a.test").expect("an address")
}

fn store_b() -> StoreAddress {
    StoreAddress::new("https://store-b.test").expect("an address")
}

/// Where a profile keeps what it holds for `address`.
fn folder_of(home: &TempDir, account: u64, address: &StoreAddress) -> PathBuf {
    crate::project_store::store_directory(&profile_dir(home, account), address)
}

fn read(folder: &Path, file: &str) -> String {
    std::fs::read_to_string(folder.join(file)).unwrap_or_else(|why| panic!("{} has no {file}: {why}", folder.display()))
}

/// A rig whose forge has granted `octocat`, signed in and settled.
fn signed_in_octocat() -> Rig {
    let rig = rig(FakeSignIn::new().grants("tok-secret-1", octocat()));
    rig.signing.start(begin(), &[]).expect("begun");
    settled(&rig.signing);
    rig
}

#[test]
fn a_full_sign_in_leaves_one_profile_no_device_and_no_token_on_disk_and_the_store_is_never_asked() {
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

    let directory = profile_dir(&rig.home, 583_231);
    assert!(directory.is_dir(), "the profile has its directory");
    assert!(files_under(&directory).is_empty(), "no key, no device id, no address: {:?}", files_under(&directory));
    assert!(!directory.join("stores").exists());
    assert!(rig.store.store.devices().is_empty());
    assert!(rig.store.store.calls().is_empty(), "the store was never asked: {:?}", rig.store.store.calls());
    assert!(rig.store.enrolled_at().is_empty() && rig.store.tokens().is_empty());
    assert!(rig.store.logins_asked().is_empty(), "gh was never asked for a token");

    for secret in ["tok-secret-1", "fake-device-code"] {
        assert!(!holds(rig.home.path(), secret), "{secret} is on disk");
    }
}

#[test]
fn the_profile_directory_is_owner_only_and_so_is_the_key_a_joined_store_leaves_in_it() {
    use std::os::unix::fs::PermissionsExt;
    let rig = signed_in_octocat();
    rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined");
    let directory = profile_dir(&rig.home, 583_231);
    let mode = |path: &Path| std::fs::metadata(path).expect("there").permissions().mode() & 0o777;
    assert_eq!(mode(&directory), 0o700);
    assert_eq!(mode(&folder_of(&rig.home, 583_231, &store_a()).join(adapters::KEY_FILE)), 0o600);
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
    first.join_store(583_231, &store_a(), "mac-a").expect("joined");
    let first_dir = folder_of(&home, 583_231, &store_a());
    let first_key = read(&first_dir, adapters::KEY_FILE);
    let first_device = read(&first_dir, adapters::ID_FILE);

    let second = sign_in(FakeSignIn::new().grants("tok-b", hubot()));
    second.start(begin(), &[]).expect("begun");
    let accounts = settled(&second);

    assert_eq!(accounts.profiles.iter().map(|p| p.login.as_str()).collect::<Vec<_>>(), ["octocat", "hubot"]);
    assert_eq!(accounts.profiles.iter().map(|p| p.chosen).collect::<Vec<_>>(), [false, true]);
    let second_profile = home.path().join(profiles::DIRECTORY).join("9919");
    assert!(files_under(&second_profile).is_empty(), "the second has joined no store: {:?}", files_under(&second_profile));
    assert_eq!(read(&first_dir, adapters::KEY_FILE), first_key);
    assert_eq!(read(&first_dir, adapters::ID_FILE), first_device);
    assert_eq!(store.store.devices(), [first_device], "signing in enrolled no one");

    second.join_store(9_919, &store_a(), "mac-a").expect("joined");
    let second_dir = crate::project_store::store_directory(&second_profile, &store_a());
    assert_ne!(read(&second_dir, adapters::KEY_FILE), first_key, "profiles never share a key");
    assert_eq!(store.store.devices().len(), 2);
    for secret in ["tok-a", "tok-b", "gh-token-of-octocat", "gh-token-of-hubot"] {
        assert!(!holds(home.path(), secret), "{secret} is on disk");
    }
}

#[test]
fn signing_in_again_as_the_same_account_is_one_profile_whose_login_is_brought_up_to_date_and_whose_store_is_left_alone() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let sign_in = |login: &str| {
        let forge = Arc::new(FakeSignIn::new().grants("tok", SignedInAs { id: 583_231, login: login.into() }));
        SigningIn::new(home.path(), forge, store.clone(), Arc::new(Ticking::from_nine())).paced(quickly())
    };
    let first = sign_in("octocat");
    first.start(begin(), &[]).expect("begun");
    settled(&first);
    let joined = first.join_store(583_231, &store_a(), "mac-a").expect("joined");
    let folder = crate::project_store::store_directory(&home.path().join(profiles::DIRECTORY).join("583231"), &store_a());
    let key = read(&folder, adapters::KEY_FILE);

    let again = sign_in("octocat-renamed");
    again.start(begin(), &[]).expect("begun");
    let accounts = settled(&again);

    assert_eq!(accounts.profiles.len(), 1, "the same account is the same profile");
    assert_eq!(accounts.profiles[0].login, "octocat-renamed", "the login is brought up to date");
    assert_eq!(read(&folder, adapters::KEY_FILE), key, "the key is the profile's own");
    assert_eq!(read(&folder, adapters::ID_FILE), joined, "signing in again enrols nothing, so the device stands");
    assert_eq!(store.store.devices(), [joined]);
    assert!(store.deleted_at().is_empty(), "no device was signed out");
    assert_eq!(store.logins_asked(), ["octocat"], "gh was asked once, for the join");
}

/// Both accounts signed in on one machine, over the one `store`. Answers the second's sign-in,
/// which reads the same list as the first's.
fn both_signed_in(home: &TempDir, store: &Arc<Enrolled>) -> SigningIn {
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
fn signing_out_removes_only_that_profile_and_signs_its_device_out_of_each_store_it_joined() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = both_signed_in(&home, &store);
    let in_a = signing.join_store(583_231, &store_a(), "mac-a").expect("joined");
    let in_b = signing.join_store(583_231, &store_b(), "mac-a").expect("joined");
    let kept = signing.join_store(9_919, &store_a(), "mac-a").expect("joined");
    let kept_folder = folder_of(&home, 9_919, &store_a());
    let kept_key = read(&kept_folder, adapters::KEY_FILE);

    let out = signing.sign_out(583_231, &[]).expect("signed out");

    assert!(!out.store_row_remains && out.said.is_none());
    assert_eq!(out.accounts.profiles.len(), 1);
    assert_eq!(out.accounts.profiles[0].login, "hubot");
    assert!(!profile_dir(&home, 583_231).exists(), "its directory is gone");
    assert_eq!(
        store.deleted_at(),
        [(store_a().as_str().to_string(), in_a), (store_b().as_str().to_string(), in_b)],
        "its device was signed out of each store it joined, and no one else's"
    );
    assert_eq!(store.store.devices(), [kept.clone()]);
    assert_eq!(read(&kept_folder, adapters::KEY_FILE), kept_key);
    assert_eq!(read(&kept_folder, adapters::ID_FILE), kept);
}

#[test]
fn signing_out_with_a_store_down_still_removes_the_profile_and_says_its_row_remains() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = both_signed_in(&home, &store);
    signing.join_store(583_231, &store_a(), "mac-a").expect("joined");
    signing.join_store(9_919, &store_a(), "mac-a").expect("joined");
    store.store.set_unreachable(true);

    let out = signing.sign_out(9_919, &[]).expect("signed out");

    assert!(out.store_row_remains);
    let said = out.said.expect("it says why");
    assert!(said.contains("could not be reached") && said.contains(store_a().as_str()), "{said}");
    assert_eq!(out.accounts.profiles.len(), 1);
    assert!(!profile_dir(&home, 9_919).exists());
    store.store.set_unreachable(false);
    assert_eq!(store.store.devices().len(), 2, "the store was not changed");
}

#[test]
fn signing_out_of_a_profile_that_joined_no_store_removes_it_and_asks_no_store() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = both_signed_in(&home, &store);

    let out = signing.sign_out(9_919, &[]).expect("signed out");

    assert!(!out.store_row_remains && out.said.is_none(), "{:?}", out.said);
    assert_eq!(out.accounts.profiles.len(), 1);
    assert!(!profile_dir(&home, 9_919).exists());
    assert!(store.deleted_at().is_empty() && store.store.calls().is_empty(), "no store was asked");
}

#[test]
fn signing_out_of_an_account_the_machine_never_joined_is_refused_and_changes_nothing() {
    let home = TempDir::new();
    let store = Arc::new(Enrolled::new());
    let signing = both_signed_in(&home, &store);
    assert_eq!(signing.sign_out(1, &[]), Err(Refused::NoSuchAccount(1)));
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
fn a_blank_sign_in_app_refuses_in_words_that_name_it_and_asks_the_forge_nothing() {
    let rig = rig(FakeSignIn::new());
    for client in ["", "   "] {
        let Refused::Unavailable(said) = rig.signing.start(Begin { client_id: client.into() }, &[]).unwrap_err() else {
            panic!("not refused as unavailable")
        };
        assert_eq!(said, "Sign-in is unavailable: the sign-in app is blank in Settings.");
    }
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

#[test]
fn a_join_enrols_once_with_the_logins_token_writes_the_stores_address_and_never_the_token() {
    let rig = signed_in_octocat();
    let device = rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined");

    let folder = folder_of(&rig.home, 583_231, &store_a());
    let name = folder.file_name().and_then(|name| name.to_str()).expect("a name");
    assert!(name.len() == 16 && name.bytes().all(|byte| byte.is_ascii_hexdigit()), "{name}");
    assert!(device.starts_with(&format!("{name}.")), "{device}");
    assert_eq!(read(&folder, "address"), store_a().as_str());
    assert_eq!(read(&folder, adapters::ID_FILE), device);
    assert!(!read(&folder, adapters::KEY_FILE).is_empty());
    assert_eq!(rig.store.logins_asked(), ["octocat"], "gh's token was asked for the profile's login");
    assert_eq!(rig.store.tokens(), ["gh-token-of-octocat"], "the store was given that token once");
    assert_eq!(rig.store.enrolled_at(), [store_a().as_str()]);
    assert_eq!(*rig.store.machines.lock().unwrap(), ["mac-a"]);
    assert_eq!(rig.store.store.devices(), [device.clone()]);
    for secret in ["gh-token-of-octocat", "tok-secret-1"] {
        assert!(!holds(rig.home.path(), secret), "{secret} is on disk");
    }

    let again = rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined again");
    assert_eq!(again, device, "a profile already there answers its device");
    assert_eq!(rig.store.logins_asked().len(), 1, "and asks for no token");
    assert_eq!((rig.store.tokens().len(), rig.store.enrolled_at().len()), (1, 1), "nor enrols again");
    assert_eq!(rig.store.store.devices(), [device]);
}

#[test]
fn two_stores_make_two_device_folders_under_one_profile() {
    let rig = signed_in_octocat();
    let in_a = rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined");
    let in_b = rig.signing.join_store(583_231, &store_b(), "mac-a").expect("joined");

    let (a, b) = (folder_of(&rig.home, 583_231, &store_a()), folder_of(&rig.home, 583_231, &store_b()));
    assert_ne!(a, b);
    assert_eq!((read(&a, "address"), read(&b, "address")), (store_a().as_str().to_string(), store_b().as_str().to_string()));
    assert_eq!((read(&a, adapters::ID_FILE), read(&b, adapters::ID_FILE)), (in_a.clone(), in_b.clone()));
    assert_ne!(read(&a, adapters::KEY_FILE), read(&b, adapters::KEY_FILE), "each store has a key of its own");
    let folders = std::fs::read_dir(profile_dir(&rig.home, 583_231).join("stores")).expect("a stores folder").count();
    assert_eq!(folders, 2);
    assert_eq!(rig.store.store.devices().len(), 2);
    assert_eq!(rig.store.enrolled_at(), [store_a().as_str(), store_b().as_str()]);
    assert_eq!(rig.store.logins_asked().len(), 2, "a token is read for each enrolment, and never kept between");
}

#[test]
fn a_join_for_an_account_the_machine_never_joined_is_refused_and_asks_neither_gh_nor_a_store() {
    let rig = signed_in_octocat();
    let Err(StoreError::Refused(said)) = rig.signing.join_store(1, &store_a(), "mac-a") else { panic!("not refused") };
    assert!(said.contains("not signed in as account 1"), "{said}");
    assert!(rig.store.logins_asked().is_empty() && rig.store.enrolled_at().is_empty());
    assert!(!profile_dir(&rig.home, 1).exists());
}

#[test]
fn a_join_with_gh_not_signed_in_leaves_no_device_and_asks_no_store_and_a_later_join_works() {
    let rig = signed_in_octocat();
    rig.store.gh_signed_out.store(true, Ordering::SeqCst);

    let Err(StoreError::Refused(said)) = rig.signing.join_store(583_231, &store_a(), "mac-a") else { panic!("not refused") };
    assert!(said.contains("gh is not signed in as octocat"), "{said}");

    assert_eq!(rig.store.logins_asked(), ["octocat"]);
    assert!(rig.store.enrolled_at().is_empty() && rig.store.tokens().is_empty(), "no store was asked");
    assert!(rig.store.store.devices().is_empty());
    // What is left is the empty folder `join_store` made before it asked `gh`.
    assert!(files_under(&profile_dir(&rig.home, 583_231)).is_empty(), "{:?}", files_under(&profile_dir(&rig.home, 583_231)));
    assert_eq!(adapters::device_id(&folder_of(&rig.home, 583_231, &store_a())), Ok(None));

    rig.store.gh_signed_out.store(false, Ordering::SeqCst);
    let device = rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined once gh is signed in");
    assert_eq!(rig.store.store.devices(), [device]);
}

#[test]
fn a_store_that_refuses_the_enrolment_leaves_no_device_and_no_address_and_the_profile_can_join_again() {
    let rig = signed_in_octocat();
    rig.store.refusing.store(true, Ordering::SeqCst);

    let Err(StoreError::Refused(said)) = rig.signing.join_store(583_231, &store_a(), "mac-a") else { panic!("not refused") };
    assert!(said.contains("refused the token"), "{said}");

    let folder = folder_of(&rig.home, 583_231, &store_a());
    assert_eq!(adapters::device_id(&folder), Ok(None), "no device id was kept");
    assert!(!folder.join("address").exists(), "the store is not written down as joined");
    assert!(rig.store.store.devices().is_empty());
    assert!(!holds(rig.home.path(), "gh-token-of-octocat"), "the token is on disk");
    // True of the code as it is: the key is made before the store is asked, and a refusal leaves
    // it in the folder with no device beside it.
    let key = read(&folder, adapters::KEY_FILE);
    assert_eq!(files_under(&folder), [folder.join(adapters::KEY_FILE)]);

    rig.store.refusing.store(false, Ordering::SeqCst);
    let device = rig.signing.join_store(583_231, &store_a(), "mac-a").expect("joined once the store accepts");
    assert_eq!(read(&folder, adapters::KEY_FILE), key, "the retry enrols the key already there");
    assert_eq!(read(&folder, "address"), store_a().as_str());
    assert_eq!(rig.store.store.devices(), [device]);
}

#[test]
fn signing_out_after_joins_that_failed_asks_no_store_for_a_device_it_never_got() {
    let rig = signed_in_octocat();
    rig.store.gh_signed_out.store(true, Ordering::SeqCst);
    rig.signing.join_store(583_231, &store_a(), "mac-a").unwrap_err();
    rig.store.gh_signed_out.store(false, Ordering::SeqCst);
    rig.store.refusing.store(true, Ordering::SeqCst);
    rig.signing.join_store(583_231, &store_b(), "mac-a").unwrap_err();

    let out = rig.signing.sign_out(583_231, &[]).expect("signed out");

    assert!(!out.store_row_remains && out.said.is_none(), "{:?}", out.said);
    assert!(rig.store.deleted_at().is_empty() && rig.store.store.calls().is_empty(), "no store was asked");
    assert!(!profile_dir(&rig.home, 583_231).exists());
}
