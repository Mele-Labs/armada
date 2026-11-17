//! What the owner chose a page may do, kept where Armada Classic keeps it (`site-permission-store.ts`, `shared/site-permissions.ts`).
//!
//! **Native reads and writes Classic's own file**, `site-permissions.json` in its user data folder, so what he allowed in either app
//! stays allowed. The choices are kept for each repository (the manifest id, else `none`), with one set of Mac-wide defaults beside them:
//!
//! ```json
//! { "armada": { "sites": { "example.com": { "camera": "allow" } } }, "defaults": { "popups": "block" } }
//! ```
//!
//! Classic may write the file while Bridge runs, so every write reads it again first and changes only the one value it was asked to,
//! and a key this code does not know stays as it was. A read is the whole file, tiny, and is done off the main thread by whoever calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

/// The folder name Classic's user data takes from its package name (`@armada/desktop`): Electron's `userData`, not Bridge's own.
const CLASSIC_USER_DATA: &str = "@armada/desktop";
const FILE_NAME: &str = "site-permissions.json";
/// Where the Mac-wide defaults sit in the file, beside the repositories.
const DEFAULTS: &str = "defaults";
/// The repository key of a Job that has no manifest.
pub const NO_REPOSITORY: &str = "none";

/// What a page may ask for, in the order they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    LocalNetwork,
    Camera,
    Microphone,
    Notifications,
    Clipboard,
    Location,
    Downloads,
    Popups,
    Fullscreen,
}

impl Permission {
    /// The nine, in the order they are listed.
    pub const ALL: [Permission; 9] = [
        Permission::LocalNetwork,
        Permission::Camera,
        Permission::Microphone,
        Permission::Notifications,
        Permission::Clipboard,
        Permission::Location,
        Permission::Downloads,
        Permission::Popups,
        Permission::Fullscreen,
    ];

    /// The name in the file.
    pub fn key(self) -> &'static str {
        match self {
            Permission::LocalNetwork => "local_network",
            Permission::Camera => "camera",
            Permission::Microphone => "microphone",
            Permission::Notifications => "notifications",
            Permission::Clipboard => "clipboard",
            Permission::Location => "location",
            Permission::Downloads => "downloads",
            Permission::Popups => "popups",
            Permission::Fullscreen => "fullscreen",
        }
    }

    pub fn from_key(key: &str) -> Option<Permission> {
        Permission::ALL.into_iter().find(|one| one.key() == key)
    }

    /// What the person reads.
    pub fn label(self) -> &'static str {
        match self {
            Permission::LocalNetwork => "Local network",
            Permission::Camera => "Camera",
            Permission::Microphone => "Microphone",
            Permission::Notifications => "Notifications",
            Permission::Clipboard => "Clipboard",
            Permission::Location => "Location",
            Permission::Downloads => "Downloads",
            Permission::Popups => "Popups",
            Permission::Fullscreen => "Fullscreen",
        }
    }

    /// What every site is given for it before anyone chose (`NO_SETTINGS.defaults`).
    pub fn shipped(self) -> Choice {
        match self {
            Permission::Popups | Permission::Fullscreen => Choice::Allow,
            _ => Choice::Ask,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Ask,
    Allow,
    Block,
}

impl Choice {
    /// The three, in the order they are listed.
    pub const ALL: [Choice; 3] = [Choice::Ask, Choice::Allow, Choice::Block];

    pub fn key(self) -> &'static str {
        match self {
            Choice::Ask => "ask",
            Choice::Allow => "allow",
            Choice::Block => "block",
        }
    }

    pub fn from_key(key: &str) -> Option<Choice> {
        Choice::ALL.into_iter().find(|one| one.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Choice::Ask => "Ask",
            Choice::Allow => "Allow",
            Choice::Block => "Block",
        }
    }
}

/// What the person says to a request. Esc is `Dismiss`, which refuses it now and keeps nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Allow,
    Block,
    Dismiss,
}

impl Answer {
    /// The choice an answer is remembered as: a dismissal is not.
    pub fn remembered(self) -> Option<Choice> {
        match self {
            Answer::Allow => Some(Choice::Allow),
            Answer::Block => Some(Choice::Block),
            Answer::Dismiss => None,
        }
    }
}

/// The file's text as a tree, or none where it is not JSON.
fn tree_of(text: &str) -> Option<Value> {
    ipc::decode::<Value>("site-permissions.json", text.as_bytes()).ok()
}

/// The choices as the file holds them: for each repository, for each host, for each permission, and the saved defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settings {
    sites: HashMap<String, HashMap<String, HashMap<Permission, Choice>>>,
    saved: HashMap<Permission, Choice>,
}

fn choice_in(value: &Value, permission: Permission) -> Option<Choice> {
    value.get(permission.key()).and_then(Value::as_str).and_then(Choice::from_key)
}

impl Settings {
    /// The choices in a file's text. A file that is not an object, or does not parse, is no choices at all, as Classic starts again.
    pub fn parse(text: &str) -> Settings {
        let Some(Value::Object(file)) = tree_of(text) else { return Settings::default() };
        Settings::of(&file)
    }

    fn of(file: &Map<String, Value>) -> Settings {
        let mut settings = Settings::default();
        for (key, one) in file {
            if key == DEFAULTS {
                for permission in Permission::ALL {
                    if let Some(choice) = choice_in(one, permission) {
                        settings.saved.insert(permission, choice);
                    }
                }
                continue;
            }
            let Some(Value::Object(sites)) = one.get("sites") else { continue };
            let mut hosts = HashMap::new();
            for (host, choices) in sites {
                let mut kept = HashMap::new();
                for permission in Permission::ALL {
                    if let Some(choice) = choice_in(choices, permission) {
                        kept.insert(permission, choice);
                    }
                }
                hosts.insert(host.clone(), kept);
            }
            settings.sites.insert(key.clone(), hosts);
        }
        settings
    }

    /// What every site without a choice of its own is given: what was saved over the shipped ones.
    pub fn default_of(&self, permission: Permission) -> Choice {
        self.saved.get(&permission).copied().unwrap_or_else(|| permission.shipped())
    }

    /// The choice the host itself has made, if it has.
    pub fn own(&self, repository: &str, host: &str, permission: Permission) -> Option<Choice> {
        self.sites.get(repository)?.get(host)?.get(&permission).copied()
    }

    /// What a host is given for a permission: its own choice, else the default (`effectiveChoice`).
    pub fn effective(&self, repository: &str, host: &str, permission: Permission) -> Choice {
        self.own(repository, host, permission).unwrap_or_else(|| self.default_of(permission))
    }

    /// The hosts of a repository that chose `choice` for `permission` for themselves, in order.
    pub fn hosts_with(&self, repository: &str, permission: Permission, choice: Choice) -> Vec<String> {
        let mut hosts: Vec<String> = self
            .sites
            .get(repository)
            .map(|hosts| hosts.iter().filter(|(_, own)| own.get(&permission) == Some(&choice)).map(|(host, _)| host.clone()).collect())
            .unwrap_or_default();
        hosts.sort();
        hosts
    }
}

/// The object under `key`, made where it is missing or is something else.
fn object_in<'a>(parent: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let slot = parent.entry(key.to_string()).or_insert_with(|| Value::Object(Map::new()));
    if !slot.is_object() {
        *slot = Value::Object(Map::new());
    }
    match slot {
        Value::Object(object) => object,
        _ => unreachable!("made an object above"),
    }
}

/// `choose`, on the file's whole tree: sets one value and leaves everything else, whatever it is.
fn choose_in(file: &mut Map<String, Value>, repository: &str, host: &str, permission: Permission, choice: Choice) {
    let one = object_in(file, repository);
    let sites = object_in(one, "sites");
    let choices = object_in(sites, host);
    choices.insert(permission.key().to_string(), Value::String(choice.key().to_string()));
}

/// `setDefault`, on the file's whole tree.
fn set_default_in(file: &mut Map<String, Value>, permission: Permission, choice: Choice) {
    object_in(file, DEFAULTS).insert(permission.key().to_string(), Value::String(choice.key().to_string()));
}

/// Writers of this app, one at a time: two in this process never read the same file and write over each other.
static WRITING: Mutex<()> = Mutex::new(());

/// The file, as one place that reads and writes it. Every call blocks on the disk: callers run it on a background thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// Classic's own file, `~/Library/Application Support/@armada/desktop/site-permissions.json`.
    pub fn classic() -> Option<Store> {
        let home = std::env::var_os("HOME")?;
        Some(Store { path: Path::new(&home).join("Library").join("Application Support").join(CLASSIC_USER_DATA).join(FILE_NAME) })
    }

    pub fn at(path: PathBuf) -> Store {
        Store { path }
    }

    /// The choices now. No file is no choices.
    pub fn read(&self) -> Settings {
        std::fs::read_to_string(&self.path).map(|text| Settings::parse(&text)).unwrap_or_default()
    }

    /// The file's tree, as it is now: an object even where the file is missing or is not one.
    fn tree(&self) -> Map<String, Value> {
        match std::fs::read_to_string(&self.path).ok().and_then(|text| tree_of(&text)) {
            Some(Value::Object(file)) => file,
            _ => Map::new(),
        }
    }

    /// Written whole beside the file and moved into place, so Classic never reads half of it.
    fn save(&self, file: &Map<String, Value>) -> std::io::Result<()> {
        if let Some(folder) = self.path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let beside = self.path.with_extension("json.bridge-native");
        std::fs::write(&beside, Value::Object(file.clone()).to_string())?;
        std::fs::rename(beside, &self.path)
    }

    /// The host's choice for a permission is kept for the repository. Answers the choices as written.
    pub fn choose(&self, repository: &str, host: &str, permission: Permission, choice: Choice) -> std::io::Result<Settings> {
        let _one = WRITING.lock().unwrap_or_else(|held| held.into_inner());
        let mut file = self.tree();
        choose_in(&mut file, repository, host, permission, choice);
        self.save(&file)?;
        Ok(Settings::of(&file))
    }

    /// The Mac-wide default for a permission. Answers the choices as written.
    pub fn set_default(&self, permission: Permission, choice: Choice) -> std::io::Result<Settings> {
        let _one = WRITING.lock().unwrap_or_else(|held| held.into_inner());
        let mut file = self.tree();
        set_default_in(&mut file, permission, choice);
        self.save(&file)?;
        Ok(Settings::of(&file))
    }
}

/// The host of a web address, with its port where it has one, or none for anything but http and https (`hostOf`).
pub fn host_of(address: &str) -> Option<String> {
    let (scheme, rest) = address.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // The sign-in before an `@` is not the host.
    let host = authority.rsplit('@').next().unwrap_or_default().to_ascii_lowercase();
    // `new URL(..).host` leaves a scheme's own port off.
    let own = if scheme.eq_ignore_ascii_case("http") { ":80" } else { ":443" };
    let host = host.strip_suffix(own).map(str::to_string).unwrap_or(host);
    (!host.is_empty()).then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLASSIC: &str = r#"{"armada":{"sites":{"192.168.68.98:5481":{"local_network":"allow"}}},"defaults":{}}"#;

    fn tree(text: &str) -> Map<String, Value> {
        match tree_of(text) {
            Some(Value::Object(file)) => file,
            other => panic!("not a tree: {other:?}"),
        }
    }

    #[test]
    fn nothing_chosen_gives_the_shipped_defaults() {
        let none = Settings::default();
        let asked: Vec<_> = Permission::ALL.iter().map(|one| none.effective("armada", "example.com", *one)).collect();
        assert_eq!(
            asked,
            [Choice::Ask, Choice::Ask, Choice::Ask, Choice::Ask, Choice::Ask, Choice::Ask, Choice::Ask, Choice::Allow, Choice::Allow],
            "popups and fullscreen are allowed, the rest ask"
        );
    }

    #[test]
    fn the_nine_are_listed_in_classics_order_with_its_words() {
        let words: Vec<_> = Permission::ALL.iter().map(|one| (one.key(), one.label())).collect();
        assert_eq!(
            words,
            [
                ("local_network", "Local network"),
                ("camera", "Camera"),
                ("microphone", "Microphone"),
                ("notifications", "Notifications"),
                ("clipboard", "Clipboard"),
                ("location", "Location"),
                ("downloads", "Downloads"),
                ("popups", "Popups"),
                ("fullscreen", "Fullscreen"),
            ]
        );
        assert_eq!(Choice::ALL.map(Choice::label), ["Ask", "Allow", "Block"]);
    }

    #[test]
    fn a_file_classic_wrote_is_read_and_a_site_choice_beats_the_default() {
        let settings = Settings::parse(CLASSIC);
        assert_eq!(settings.effective("armada", "192.168.68.98:5481", Permission::LocalNetwork), Choice::Allow);
        assert_eq!(settings.effective("armada", "example.com", Permission::LocalNetwork), Choice::Ask, "another host has no choice");
        assert_eq!(settings.effective("other", "192.168.68.98:5481", Permission::LocalNetwork), Choice::Ask, "another repository has no choice");
        let defaults = Settings::parse(r#"{"defaults":{"camera":"block","popups":"ask","fullscreen":"nonsense"}}"#);
        assert_eq!(defaults.effective("none", "a.com", Permission::Camera), Choice::Block);
        assert_eq!(defaults.effective("none", "a.com", Permission::Popups), Choice::Ask);
        assert_eq!(defaults.effective("none", "a.com", Permission::Fullscreen), Choice::Allow, "a choice that is not one is the shipped default");
    }

    #[test]
    fn a_file_that_is_not_choices_is_no_choices() {
        for text in ["", "not json", "[1,2]", "null", r#"{"armada":5,"defaults":7}"#] {
            assert_eq!(Settings::parse(text), Settings::default(), "{text}");
        }
    }

    #[test]
    fn choosing_sets_one_value_and_keeps_every_key_it_does_not_know() {
        let mut file = tree(r#"{"armada":{"sites":{"a.com":{"camera":"allow","tomorrow":"x"}},"extra":1},"defaults":{"popups":"block","new":"y"},"somewhere":[1]}"#);
        choose_in(&mut file, "armada", "a.com", Permission::Microphone, Choice::Block);
        choose_in(&mut file, "armada", "b.com", Permission::Camera, Choice::Allow);
        choose_in(&mut file, "fresh", "c.com", Permission::Location, Choice::Block);
        set_default_in(&mut file, Permission::Camera, Choice::Block);
        let after = Value::Object(file);
        assert_eq!(after["armada"]["extra"], 1);
        assert_eq!(after["armada"]["sites"]["a.com"]["tomorrow"], "x");
        assert_eq!(after["armada"]["sites"]["a.com"]["camera"], "allow");
        assert_eq!(after["armada"]["sites"]["a.com"]["microphone"], "block");
        assert_eq!(after["armada"]["sites"]["b.com"]["camera"], "allow");
        assert_eq!(after["fresh"]["sites"]["c.com"]["location"], "block");
        assert_eq!(after["defaults"]["popups"], "block");
        assert_eq!(after["defaults"]["new"], "y");
        assert_eq!(after["defaults"]["camera"], "block");
        assert_eq!(after["somewhere"][0], 1);
    }

    #[test]
    fn a_write_reads_the_file_again_so_what_classic_wrote_since_stays() {
        let folder = std::env::temp_dir().join(format!("armada-site-choices-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let store = Store::at(folder.join("site-permissions.json"));
        assert_eq!(store.read(), Settings::default(), "no file is no choices");
        store.choose("armada", "a.com", Permission::Camera, Choice::Allow).expect("written");
        // Classic writes a key of its own and a default between two of ours.
        std::fs::write(&store.path, r#"{"armada":{"sites":{"a.com":{"camera":"allow"}}},"classics":1,"defaults":{"popups":"block"}}"#).expect("classic writes");
        let kept = store.set_default(Permission::Camera, Choice::Block).expect("written");
        let text = std::fs::read_to_string(&store.path).expect("read");
        assert!(text.contains("\"classics\""), "{text}");
        assert_eq!(kept.effective("armada", "a.com", Permission::Camera), Choice::Allow);
        assert_eq!(kept.default_of(Permission::Camera), Choice::Block);
        assert_eq!(kept.default_of(Permission::Popups), Choice::Block, "the default Classic wrote since is kept");
        assert_eq!(store.read(), kept);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_hosts_own_choices_are_listed_by_what_they_chose() {
        let settings = Settings::parse(r#"{"armada":{"sites":{"b.com":{"camera":"allow"},"a.com":{"camera":"allow"},"c.com":{"camera":"block"}}}}"#);
        assert_eq!(settings.hosts_with("armada", Permission::Camera, Choice::Allow), ["a.com", "b.com"]);
        assert!(settings.hosts_with("armada", Permission::Microphone, Choice::Allow).is_empty());
    }

    #[test]
    fn the_host_is_the_authority_of_a_web_address_and_nothing_else_has_one() {
        assert_eq!(host_of("https://Example.com/a?b#c").as_deref(), Some("example.com"));
        assert_eq!(host_of("http://localhost:3000/x").as_deref(), Some("localhost:3000"));
        assert_eq!(host_of("https://user:pw@example.com:8443/").as_deref(), Some("example.com:8443"));
        assert_eq!(host_of("https://example.com:443/").as_deref(), Some("example.com"), "the scheme's own port is not part of the host");
        assert_eq!(host_of("http://example.com:443/").as_deref(), Some("example.com:443"));
        assert_eq!(host_of("file:///etc/hosts"), None);
        assert_eq!(host_of("about:blank"), None);
        assert_eq!(host_of("https:///x"), None);
    }

    #[test]
    fn an_answer_is_remembered_as_its_choice_and_a_dismissal_is_not() {
        assert_eq!(Answer::Allow.remembered(), Some(Choice::Allow));
        assert_eq!(Answer::Block.remembered(), Some(Choice::Block));
        assert_eq!(Answer::Dismiss.remembered(), None);
    }
}
