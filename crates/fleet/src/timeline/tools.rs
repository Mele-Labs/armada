//! What a Session read, wrote and ran, entered one per burst and not one per tool call, and what it
//! called outside the repository, entered one per call.
//!
//! **A tool call is a fact an agent makes hundreds of times an hour**, so each is counted here and
//! the count is entered once the burst is over: `Read 14 files`, `Edited 3 files`, `Ran 5 commands`,
//! each with the latest path or command as its detail and the newest [`ipc::MOST_TIMELINE_ITEMS`] of
//! them as the items it opens to, the detail first. A burst is one subject's run of one act; it
//! ends when that subject has done none of it for [`QUIET`], when its turn ends, or [`MOST`] after
//! it began, so a long run shows as it goes. **The entry is dated by the burst's first call** and is
//! written when it closes, which is why a client orders by `at` and not by `id`.
//!
//! A path is the only thing kept of a file. A command is kept cut and with what looks like a
//! secret in it hidden ([`scrubbed`]).
//!
//! **A call to a service outside the repository is not a burst**: a forge command, a page fetched, a
//! search, another tool's server. Each is its own `session.called` entry, since what a person follows
//! is which service was reached and when. The harness and the adapters say which calls these are and
//! hand over a [`Reach`] that names the act and never what it was given.

use std::collections::BTreeMap;
use std::time::Duration;

use adapter_traits::{AgentHarness, DroneEvent, Reach, ToolUse};
use ipc::{TimelineKind, TimelineTone};

use super::{bare, Fact, Subject, Timeline, DETAIL};

/// A burst is over when its subject has done none of that act for this long, in milliseconds.
const QUIET: i64 = 15_000;
/// A burst is entered at most this long after it began, in milliseconds, whatever it is still doing.
const MOST: i64 = 120_000;
/// How often the writer looks for a burst that is over.
pub(super) const TICK: Duration = Duration::from_secs(5);

/// What was done.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Act {
    Read,
    Edit,
    Ran,
}

/// Who did it, as the entry will name them.
#[derive(Clone, Debug)]
pub(crate) struct Doer {
    pub(crate) subject: Subject,
    pub(crate) title: Option<String>,
    pub(crate) manifest: Option<String>,
}

impl Doer {
    pub(crate) fn session(id: &str, title: Option<String>, manifest: Option<String>) -> Doer {
        Doer { subject: Subject::Session(id.to_string()), title, manifest }
    }

    pub(crate) fn of_session(session: &store::KeptSession) -> Doer {
        Doer::session(&session.id, session.title.clone(), session.manifest_id.clone())
    }

    fn key(&self) -> String {
        match &self.subject {
            Subject::Session(id) => format!("session:{id}"),
            Subject::Pull(number, _) => format!("pull:{number}"),
            Subject::Main => "main".to_string(),
        }
    }
}

struct Burst {
    doer: Doer,
    began_at: String,
    began: i64,
    last: i64,
    count: usize,
    /// What it named, newest first and without repeats, cut to a line each: the burst's `detail` is the first.
    items: Vec<String>,
}

/// The bursts open now, by subject and act.
#[derive(Default)]
pub(crate) struct Bursts {
    open: BTreeMap<(String, Act), Burst>,
}

impl Bursts {
    fn note(&mut self, doer: Doer, act: Act, at: &str, ms: i64, count: usize, detail: Option<String>) {
        let burst = self.open.entry((doer.key(), act)).or_insert_with(|| Burst {
            doer,
            began_at: at.to_string(),
            began: ms,
            last: ms,
            count: 0,
            items: Vec::new(),
        });
        burst.count += count;
        burst.last = ms;
        // What a repeat names moves to the front rather than listing twice, so the list holds distinct files.
        if let Some(item) = detail.map(|one| bare(&one, DETAIL)).filter(|one| !one.is_empty()) {
            burst.items.retain(|held| *held != item);
            burst.items.insert(0, item);
            burst.items.truncate(ipc::MOST_TIMELINE_ITEMS);
        }
    }

    /// The bursts that are over at `now`, quiet or long enough.
    fn due(&mut self, now: i64) -> Vec<Fact> {
        let over: Vec<_> = self
            .open
            .iter()
            .filter(|(_, one)| now - one.last >= QUIET || now - one.began >= MOST)
            .map(|(key, _)| key.clone())
            .collect();
        self.entered(over)
    }

    /// Every burst of one subject, because its turn or its life ended.
    fn closed(&mut self, doer: &Doer) -> Vec<Fact> {
        let key = doer.key();
        let over: Vec<_> = self.open.keys().filter(|(held, _)| *held == key).cloned().collect();
        self.entered(over)
    }

    fn entered(&mut self, keys: Vec<(String, Act)>) -> Vec<Fact> {
        keys.into_iter()
            .filter_map(|key| {
                let burst = self.open.remove(&key)?;
                Some(fact(key.1, burst))
            })
            .collect()
    }
}

/// A burst as the entry it becomes.
fn fact(act: Act, burst: Burst) -> Fact {
    let (kind, verb, noun) = match act {
        Act::Read => (TimelineKind::SessionRead, "Read", "file"),
        Act::Edit => (TimelineKind::SessionEdited, "Edited", "file"),
        Act::Ran => (TimelineKind::SessionRan, "Ran", "command"),
    };
    let n = burst.count;
    let says = format!("{verb} {n} {noun}{}", if n == 1 { "" } else { "s" });
    Fact::new(burst.began_at, kind, burst.doer.subject, TimelineTone::Run, says)
        .titled(burst.doer.title)
        .detail(burst.items.first().cloned())
        .items(burst.items)
        .in_repository(burst.doer.manifest)
        .live()
}

/// What a Drone's call did that the timeline enters.
pub(crate) enum Seen {
    /// A file changed or read, or a command run, and what it named.
    Did(Act, String),
    /// A service outside the repository reached.
    Called(Reach),
}

/// What a Drone's call did that the timeline enters, and what it named: a file it changed, a file it
/// read, a command it ran, a service it called. **The harness says which tools these are**, so no
/// tool is named here.
pub(crate) fn tool_of<H: AgentHarness>(harness: &H, event: &DroneEvent) -> Option<Seen> {
    if let Some(path) = harness.edited(event) {
        return Some(Seen::Did(Act::Edit, path));
    }
    Some(match harness.used(event)? {
        ToolUse::Read(path) => Seen::Did(Act::Read, path),
        ToolUse::Ran(command) => Seen::Did(Act::Ran, command),
        ToolUse::Reached(reach) => Seen::Called(reach),
    })
}

fn millis(at: &str) -> Option<i64> {
    core_model::Timestamp::from_rfc3339(at).epoch_millis()
}

impl Timeline {
    /// What `doer` just did, as [`tool_of`] saw it.
    pub(crate) fn seen(&self, doer: Doer, seen: Seen) {
        match seen {
            Seen::Did(act, what) => self.used(doer, act, 1, Some(what)),
            Seen::Called(reach) => self.called(doer, reach),
        }
    }

    /// `doer` called a service outside the repository, now: one entry, at once.
    pub(crate) fn called(&self, doer: Doer, reach: Reach) {
        self.note(called_fact(self.now(), doer, reach));
    }

    /// `count` files read or written, or one command run, by `doer`, now. **Nothing is entered
    /// until the burst closes.** `detail` is the latest path or command.
    pub(crate) fn used(&self, doer: Doer, act: Act, count: usize, detail: Option<String>) {
        let at = self.now();
        let Some(ms) = millis(&at) else { return };
        let detail = match act {
            Act::Ran => detail.map(|command| scrubbed(&command)),
            _ => detail,
        };
        if let Ok(mut bursts) = self.bursts.lock() {
            bursts.note(doer, act, &at, ms, count, detail);
        }
    }

    /// Enter every burst that is over. Called on a tick.
    pub(super) fn flush_due(&self) {
        if let Some(now) = millis(&self.now()) {
            self.flush_at(now);
        }
    }

    fn flush_at(&self, now: i64) {
        let facts = self.bursts.lock().map(|mut bursts| bursts.due(now)).unwrap_or_default();
        facts.into_iter().for_each(|fact| self.note(fact));
    }

    /// Enter every open burst of one subject: its turn ended, or it did.
    pub(crate) fn close_bursts_of(&self, doer: &Doer) {
        let facts = self.bursts.lock().map(|mut bursts| bursts.closed(doer)).unwrap_or_default();
        facts.into_iter().for_each(|fact| self.note(fact));
    }
}

/// A call as the entry it becomes: `Called {name}`, opening to the one line that says what was asked.
fn called_fact(at: String, doer: Doer, reach: Reach) -> Fact {
    Fact::new(at, TimelineKind::SessionCalled, doer.subject, TimelineTone::Run, format!("Called {}", reach.name))
        .titled(doer.title)
        .detail(Some(reach.line))
        .calling(reach.service, reach.name)
        .in_repository(doer.manifest)
        .live()
}

/// Words in a name that say the value beside it is a secret.
const SECRET_NAMES: [&str; 7] = ["token", "secret", "passw", "auth", "credential", "apikey", "api_key"];

fn names_a_secret(name: &str) -> bool {
    let name = name.trim_start_matches('-').to_lowercase();
    SECRET_NAMES.iter().any(|word| name.contains(word)) || name.ends_with("key")
}

/// A long run of letters and digits with no path in it: what a token looks like.
fn looks_like_a_token(word: &str) -> bool {
    let word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    word.len() >= 24
        && word.chars().all(|c| c.is_ascii_alphanumeric() || "_-+=.".contains(c))
        && word.chars().any(|c| c.is_ascii_digit())
        && word.chars().any(|c| c.is_ascii_alphabetic())
}

/// The value of every `name=value` in a word whose name says it is a secret, hidden: an
/// environment assignment, a `--flag=value`, a pair in an address's query.
fn hide_pairs(word: &str) -> String {
    let mut out = String::new();
    let mut pair = String::new();
    let close = |pair: &mut String, out: &mut String| {
        match pair.split_once('=') {
            Some((name, value)) if !value.is_empty() && names_a_secret(name.trim_matches(['"', '\''])) => {
                out.push_str(&format!("{name}=***"));
            }
            _ => out.push_str(pair),
        }
        pair.clear();
    };
    for c in word.chars() {
        if matches!(c, '&' | '?' | ';') {
            close(&mut pair, &mut out);
            out.push(c);
        } else {
            pair.push(c);
        }
    }
    close(&mut pair, &mut out);
    out
}

/// `command` with what looks like a secret in it hidden: the value of a `NAME=` or a `--flag` whose
/// name says it is one, what follows `Authorization:`, `Bearer` or `Basic`, a password in an address,
/// and any long token-shaped word. **It errs toward hiding**, since an entry is read by anyone who
/// can see the Cockpit.
pub(crate) fn scrubbed(command: &str) -> String {
    const HIDDEN: &str = "***";
    let mut hide = 0;
    let mut words = Vec::new();
    for word in command.split_whitespace() {
        let bare = word.trim_matches(['"', '\'']).to_lowercase();
        let paired = hide_pairs(word);
        if hide > 0 {
            hide -= 1;
            words.push(HIDDEN.to_string());
        } else if paired != word {
            words.push(paired);
        } else if bare == "authorization:" || bare == "authorization" {
            hide = 2;
            words.push(word.to_string());
        } else if bare == "bearer" || bare == "basic" || (word.starts_with('-') && names_a_secret(word)) {
            hide = 1;
            words.push(word.to_string());
        } else if let Some((scheme, rest)) = word.split_once("://").filter(|(_, rest)| rest.contains('@')) {
            let (_, host) = rest.split_once('@').unwrap_or(("", rest));
            words.push(format!("{scheme}://{HIDDEN}@{host}"));
        } else if looks_like_a_token(word) {
            words.push(HIDDEN.to_string());
        } else {
            words.push(word.to_string());
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The instant `sec` seconds into a fixed ten minutes, as text, and as the burst reads it.
    fn at(sec: i64) -> String {
        format!("2026-10-12T10:{:02}:{:02}.000Z", sec / 60, sec % 60)
    }

    fn ms(sec: i64) -> i64 {
        millis(&at(sec)).expect("an instant")
    }

    fn who(id: &str) -> Doer {
        Doer::session(id, Some("Flaky store test".into()), Some("armada".into()))
    }

    fn note(bursts: &mut Bursts, doer: Doer, act: Act, sec: i64, detail: &str) {
        bursts.note(doer, act, &at(sec), ms(sec), 1, Some(detail.to_string()));
    }

    /// Fourteen reads inside a burst are one entry, dated by the first, carrying the latest path.
    #[test]
    fn a_burst_is_one_entry_with_its_count() {
        let mut bursts = Bursts::default();
        for n in 0..14 {
            note(&mut bursts, who("s1"), Act::Read, n, &format!("src/{n}.rs"));
        }
        assert!(bursts.due(ms(13 + 14)).is_empty(), "still going");
        let entered = bursts.due(ms(13 + 15));
        assert_eq!(entered.len(), 1);
        assert_eq!(entered[0].kind_name(), "session.read");
        assert_eq!(entered[0].says(), "Read 14 files");
        assert_eq!(entered[0].detail_text(), Some("src/13.rs"));
        assert_eq!(entered[0].at(), at(0));
        assert!(bursts.due(ms(600)).is_empty(), "entered once");
    }

    /// One entry for each subject and each act, and a single call reads in the singular.
    #[test]
    fn each_subject_and_each_act_has_a_burst_of_its_own() {
        let mut bursts = Bursts::default();
        note(&mut bursts, who("s1"), Act::Read, 0, "a.rs");
        note(&mut bursts, who("s1"), Act::Edit, 0, "a.rs");
        note(&mut bursts, who("s2"), Act::Read, 0, "b.rs");
        note(&mut bursts, who("s1"), Act::Ran, 0, "cargo build");
        let mut entered: Vec<_> =
            bursts.due(ms(15)).iter().map(|f| (f.kind_name(), f.says().to_string())).collect();
        entered.sort();
        assert_eq!(
            entered,
            [
                ("session.edited".to_string(), "Edited 1 file".to_string()),
                ("session.ran".to_string(), "Ran 1 command".to_string()),
                ("session.read".to_string(), "Read 1 file".to_string()),
                ("session.read".to_string(), "Read 1 file".to_string()),
            ]
        );
    }

    /// A run that never goes quiet is still entered, so it shows as it goes.
    #[test]
    fn a_long_burst_is_entered_before_it_ends() {
        let mut bursts = Bursts::default();
        for n in 0..=130 {
            note(&mut bursts, who("s1"), Act::Ran, n, "cargo test");
        }
        let entered = bursts.due(ms(130));
        assert_eq!(entered.len(), 1);
        assert_eq!(entered[0].says(), "Ran 131 commands");
    }

    /// The burst keeps what it named, newest first and without repeats, to the cap, and the detail is the first.
    #[test]
    fn a_burst_lists_what_it_named_newest_first_to_the_cap() {
        let mut bursts = Bursts::default();
        for n in 0..40 {
            note(&mut bursts, who("s1"), Act::Read, n, &format!("src/{n}.rs"));
        }
        note(&mut bursts, who("s1"), Act::Read, 40, "src/30.rs");
        let entered = bursts.due(ms(40 + 15));
        let items = entered[0].item_texts();
        assert_eq!(entered[0].says(), "Read 41 files", "the count is the whole burst's");
        assert_eq!(items.len(), ipc::MOST_TIMELINE_ITEMS);
        assert_eq!(items[..3], ["src/30.rs", "src/39.rs", "src/38.rs"], "a repeat moves to the front");
        assert_eq!(entered[0].detail_text(), items.first().map(String::as_str));
        assert_eq!(items.iter().filter(|one| *one == "src/30.rs").count(), 1);
    }

    /// An item is cut to a line as the detail is, and one with nothing in it is not listed.
    #[test]
    fn an_item_is_one_line_and_never_empty() {
        let mut bursts = Bursts::default();
        note(&mut bursts, who("s1"), Act::Ran, 0, "echo   one\ntwo");
        note(&mut bursts, who("s1"), Act::Ran, 1, "  ");
        note(&mut bursts, who("s1"), Act::Ran, 2, &"x".repeat(300));
        let entered = bursts.due(ms(2 + 15));
        let items = entered[0].item_texts();
        assert_eq!(entered[0].says(), "Ran 3 commands");
        assert_eq!(items.len(), 2);
        assert_eq!(items[1], "echo one two");
        assert_eq!(items[0].chars().count(), 120);
        assert_eq!(entered[0].detail_text(), Some(items[0].as_str()));
    }

    /// The end of a turn closes one subject's bursts and nobody else's.
    #[test]
    fn a_turn_ending_closes_its_own_bursts() {
        let mut bursts = Bursts::default();
        note(&mut bursts, who("s1"), Act::Read, 0, "a.rs");
        note(&mut bursts, who("s2"), Act::Read, 0, "b.rs");
        assert_eq!(bursts.closed(&who("s1")).len(), 1);
        assert_eq!(bursts.due(ms(15)).len(), 1);
    }

    /// The harness says which calls reach a service: they are heard as calls, ahead of reads and commands.
    #[test]
    fn a_call_outside_the_repository_is_heard_as_a_call_and_the_rest_as_before() {
        let harness = testkit::FakeHarness::that_listens();
        let call = |tool: &str, detail: &str| {
            DroneEvent::Called { tool: tool.into(), call: "c".into(), detail: adapter_traits::CallDetail::of(detail) }
        };
        let seen = tool_of(&harness, &call(testkit::REACHES, "web docs.rs"));
        let Some(Seen::Called(reach)) = seen else { panic!("a call") };
        assert_eq!((reach.service.as_str(), reach.line.as_str()), ("web", "docs.rs"));
        assert!(matches!(tool_of(&harness, &call(testkit::READS, "a.rs")), Some(Seen::Did(Act::Read, _))));
        assert!(matches!(tool_of(&harness, &call(testkit::RUNS, "ls")), Some(Seen::Did(Act::Ran, _))));
        assert!(matches!(tool_of(&harness, &call(testkit::EDITS, "a.rs")), Some(Seen::Did(Act::Edit, _))));
        assert!(tool_of(&harness, &call("some_other_tool", "x")).is_none());
    }

    /// A call is its own entry: the service in `call`, the line as its detail, and the Session working.
    #[test]
    fn a_call_is_one_entry_of_its_own_kind_naming_the_service() {
        let reach = Reach { service: "web".into(), name: "the web".into(), line: "docs.rs".into() };
        let fact = called_fact(at(0), who("s1"), reach);
        assert_eq!(fact.kind_name(), "session.called");
        assert_eq!(fact.says(), "Called the web");
        assert_eq!(fact.detail_text(), Some("docs.rs"));
        assert_eq!(fact.call_service(), Some("web"));
        assert_eq!(fact.at(), at(0));
        assert!(fact.item_texts().is_empty());
    }

    #[test]
    fn a_secret_in_a_command_is_hidden() {
        for (typed, shown) in [
            ("cargo test --workspace", "cargo test --workspace"),
            ("API_TOKEN=abc123 deploy now", "API_TOKEN=*** deploy now"),
            ("curl -H \"Authorization: Bearer abc123\" https://x", "curl -H \"Authorization: *** *** https://x"),
            ("deploy --password hunter2 --env prod", "deploy --password *** --env prod"),
            ("git clone https://me:pw@host/repo.git", "git clone https://***@host/repo.git"),
            ("tool --api-key=xyz", "tool --api-key=***"),
            ("curl https://x/y?id=1&access_token=zzz", "curl https://x/y?id=1&access_token=***"),
            ("echo a1B2c3D4e5F6g7H8i9J0k1L2m3N4o5P6", "echo ***"),
            ("cat docs/practices/protocol.md", "cat docs/practices/protocol.md"),
        ] {
            assert_eq!(scrubbed(typed), shown, "{typed}");
        }
    }
}
