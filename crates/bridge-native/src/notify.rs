//! Telling the owner a Job needs him: the banner with its sound, and the Dock count.
//!
//! **Fleet works out what calls him** (`docs/practices/protocol/a-jobs-calls.md`); this collects what
//! Fleet says is news and puts it on the OS. A port of `session-attention.ts`'s client half, with the
//! rules it keeps: arrivals inside a quiet window go out as one banner, the window is not extended,
//! and no banner while a window of this app has focus or `features.notifications` is off. A press on
//! the banner brings the window forward and opens that Job.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{AnyWindowHandle, Context, Entity, Subscription, Task, WeakEntity};

use ipc::{CallArrival, CallTone};

use crate::banner;
use crate::fleet::{Arrived, FleetConnection};
use crate::jobs::JobsList;
use crate::log::Note;

/// How long arrivals are collected before one banner goes out, counted from the first.
pub const QUIET: Duration = Duration::from_millis(2000);

/// One banner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Told {
    /// The Job a press lands on.
    pub session_id: String,
    pub title: String,
    pub body: String,
    pub tone: CallTone,
    /// The ask the first arrival names, which a press opens in the review sheet. Not part of the tag: a Job's newer banner
    /// replaces its older one, whichever ask each tells.
    pub item: Option<String>,
}

/// The banner a batch of arrivals makes: the first to arrive speaks, and the rest are counted.
/// Blocked if any of them blocks, else waiting. Nothing for an empty batch.
pub fn told_of(batch: &[CallArrival]) -> Option<Told> {
    let first = batch.first()?;
    let others = batch.len() - 1;
    let title = if first.title.is_empty() { "Job".to_string() } else { first.title.clone() };
    let body = if others == 0 { first.text.clone() } else { format!("{}\nand {others} more", first.text) };
    let tone = if batch.iter().any(|one| one.tone == CallTone::Blocked) { CallTone::Blocked } else { CallTone::Waiting };
    // A Mac waiting to pair and a review asked of him name their call by its key; `session_id` carries the same for
    // readers without it.
    let by_key = matches!(first.kind, ipc::CallArrivalKind::MacPairing | ipc::CallArrivalKind::ReviewRequested);
    let tag = first.key.clone().filter(|_| by_key).unwrap_or_else(|| first.session_id.as_str().to_string());
    let item = first.item.clone().filter(|_| !by_key);
    Some(Told { session_id: tag, title, body, tone, item })
}

/// A banner's tag that is a call's key rather than a Job's id: `mac:<id>` for `CallArrivalKind::MacPairing`, or
/// a review asked of him.
pub fn is_call_key(tag: &str) -> bool {
    tag.starts_with("mac:") || is_review_key(tag)
}

/// A review asked of him, `review:<owner/name>#<number>`: `CallArrivalKind::ReviewRequested`.
pub fn is_review_key(tag: &str) -> bool {
    tag.starts_with("review:")
}

/// The sound file a tone carries, in the bundle's Resources: `crates/bridge-native/sounds`, copied in by
/// `scripts/native-app`. A name and never a player, so macOS mutes it with the banner.
pub fn sound_of(tone: CallTone) -> &'static str {
    match tone {
        CallTone::Blocked => "armada-blocked.wav",
        CallTone::Waiting => "armada-waiting.wav",
    }
}

/// Arrivals waiting for their window to close. **The window is not extended**: restarting it on each
/// arrival would hold the banner back for as long as a trickle lasted.
#[derive(Default)]
pub struct Collector {
    held: Vec<CallArrival>,
    open: bool,
}

impl Collector {
    /// Take arrivals in. True when this opened the window, which is when the caller starts the clock.
    pub fn add(&mut self, arrived: Vec<CallArrival>) -> bool {
        if arrived.is_empty() {
            return false;
        }
        self.held.extend(arrived);
        !std::mem::replace(&mut self.open, true)
    }

    /// Drop what a Job now snoozed raised while the window was open.
    pub fn without(&mut self, snoozed: impl Fn(&str) -> bool) {
        self.held.retain(|one| !snoozed(one.session_id.as_str()));
    }

    /// The window is up: the banner for everything collected, and the next arrival opens a new window.
    pub fn close(&mut self) -> Option<Told> {
        self.open = false;
        told_of(&std::mem::take(&mut self.held))
    }
}

/// Puts what Fleet says on the OS. Lives as long as the window it was made with.
pub struct Notifier {
    fleet: Entity<FleetConnection>,
    jobs: WeakEntity<JobsList>,
    window: AnyWindowHandle,
    collector: Collector,
    /// The running window's clock. Kept, or it is cancelled.
    wait: Option<Task<()>>,
    /// The count last put on the Dock, so the badge is written only when it moves.
    badge: Option<usize>,
    /// The ask each Job's standing banner tells, by its tag: what a press on it opens.
    asked: HashMap<String, String>,
    _subscriptions: Vec<Subscription>,
}

impl Notifier {
    pub fn new(fleet: &Entity<FleetConnection>, jobs: &Entity<JobsList>, window: AnyWindowHandle, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.subscribe(fleet, |this, _fleet, arrived: &Arrived, cx| this.arrive(arrived.0.clone(), cx)),
            cx.observe(fleet, |this, _fleet, cx| this.follow_count(cx)),
            // A Mac that falls asleep or wakes moves the count with no word from Fleet.
            crate::pairing::observe(cx, |this, cx| this.follow_count(cx)),
        ];
        // A press on a banner comes back through gpui. This also readies the notification centre, which
        // does nothing outside an app bundle.
        let me = cx.weak_entity();
        cx.on_system_notification_response(move |response, cx| {
            if response.action_id.is_none() {
                let _ = me.update(cx, |this, cx| this.pressed(response.tag.as_ref(), cx));
            }
        });
        let mut this = Notifier {
            fleet: fleet.clone(),
            jobs: jobs.downgrade(),
            window,
            collector: Collector::default(),
            wait: None,
            badge: None,
            asked: HashMap::new(),
            _subscriptions: subscriptions,
        };
        this.follow_count(cx);
        this
    }

    /// The Dock count follows Fleet's calls. **Not before Fleet has answered once**: an empty first
    /// reading would clear a count that stands.
    fn follow_count(&mut self, cx: &mut Context<Self>) {
        let Some(count) = self.fleet.read(cx).held().dock_count(crate::pairing::macs(cx)) else { return };
        if self.badge != Some(count) {
            self.badge = Some(count);
            banner::set_badge(count);
        }
    }

    fn arrive(&mut self, arrived: Vec<CallArrival>, cx: &mut Context<Self>) {
        if !self.collector.add(arrived) {
            return;
        }
        self.wait = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(QUIET).await;
            let _ = this.update(cx, |this, cx| this.tell(cx));
        }));
    }

    fn tell(&mut self, cx: &mut Context<Self>) {
        // A Job snoozed since its news arrived, or on a Mac asleep since, is quiet: nothing of it goes out.
        let fleet = self.fleet.read(cx).held();
        let macs = crate::pairing::macs(cx);
        self.collector.without(|job| fleet.snoozed(job) || fleet.unreachable(job, macs));
        let Some(told) = self.collector.close() else { return };
        if !self.fleet.read(cx).held().notifications {
            return;
        }
        // He is looking at the app: the row already pulses.
        if cx.active_window().is_some() {
            return;
        }
        Note::info("a banner was posted").field("tone", format!("{:?}", told.tone)).emit();
        // The newer banner replaces the Job's older one, and with it the ask a press opens.
        match told.item.clone() {
            Some(item) => self.asked.insert(told.session_id.clone(), item),
            None => self.asked.remove(&told.session_id),
        };
        banner::post(&told.session_id, &told.title, &told.body, sound_of(told.tone));
    }

    /// A press on a banner: the app comes forward and the Job opens. A Mac waiting to pair names no Job: its
    /// banner opens the Operator's page with that Mac's card in front. A review asked of him opens the pull request in
    /// Reviews, and a ready review opens it on Review comments.
    fn pressed(&mut self, session_id: &str, cx: &mut Context<Self>) {
        cx.activate(true);
        let _ = self.window.update(cx, |_view, window, _cx| window.activate_window());
        // A ready review started from its toast: Review comments on that pull request, while this run of the app holds it.
        if session_id.starts_with(crate::toasts::READY_TAG) {
            let asked = crate::toasts::ready_asked(session_id, cx);
            Note::info("a banner was pressed").field("ready", session_id).field("opened", asked.is_some()).emit();
            if let Some(asked) = asked {
                crate::operator::screen::open_findings(asked, cx);
            }
            return;
        }
        if is_review_key(session_id) {
            let asked = self.fleet.read(cx).held().review_asked(session_id).cloned();
            Note::info("a banner was pressed").field("call", session_id).field("opened", asked.is_some()).emit();
            crate::operator::screen::open_asked(asked, cx);
            return;
        }
        if is_call_key(session_id) {
            crate::operator::screen::open_call(session_id.to_string(), cx);
            Note::info("a banner was pressed").field("call", session_id).emit();
            return;
        }
        let opened = self.jobs.update(cx, |jobs, cx| jobs.open_job(session_id, cx)).unwrap_or(false);
        Note::info("a banner was pressed").field("opened", opened).emit();
        // The ask the banner told opens in the review sheet, the one place it is answered; the Job's newest where it told
        // none, or where that one no longer waits.
        let told = self.asked.remove(session_id);
        let ask = self.jobs.upgrade().and_then(|jobs| jobs.read(cx).record(session_id).and_then(|record| ask_to_open(&record.waiting_for, told.as_deref())));
        if let (true, Some(item)) = (opened, ask) {
            let item = ipc::ItemRef { session_id: ipc::SessionId::carried(session_id.to_string()), item };
            let _ = self.window.update(cx, |_, window, cx| crate::job_view::open_over(window, cx, item, None));
        }
    }
}

/// The ask a press on a Job's banner opens: the one it told while that still waits and the sheet answers it, else the newest.
pub fn ask_to_open(waiting: &[ipc::WaitingItem], told: Option<&str>) -> Option<String> {
    told.and_then(|told| waiting.iter().find(|one| one.id == told && sheet_answers(one))).map(|one| one.id.clone()).or_else(|| newest_ask(waiting))
}

/// The newest of a Job's items the review sheet answers.
pub fn newest_ask(waiting: &[ipc::WaitingItem]) -> Option<String> {
    waiting.iter().filter(|one| sheet_answers(one)).max_by(|a, b| a.since.as_str().cmp(b.since.as_str())).map(|one| one.id.clone())
}

/// An item the review sheet answers: a plan, sketch, page or draft, a question with choices, or one that names an artifact.
fn sheet_answers(one: &ipc::WaitingItem) -> bool {
    use ipc::WaitingSource::*;
    match one.source {
        Plan | Sketch | Document | Draft | AskCard => true,
        Agent => !one.options.is_empty() || one.artifact.is_some(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ipc::{CallArrivalKind, SessionId};

    fn arrival(job: &str, title: &str, text: &str, tone: CallTone) -> CallArrival {
        CallArrival { session_id: SessionId::carried(job), title: title.into(), text: text.into(), tone, kind: CallArrivalKind::Needs, key: None, item: None }
    }

    #[test]
    fn one_arrival_is_its_title_and_its_text() {
        let told = told_of(&[arrival("a", "Fix the build", "Pick a slot", CallTone::Blocked)]).expect("a banner");
        assert_eq!(told.session_id, "a");
        assert_eq!(told.title, "Fix the build");
        assert_eq!(told.body, "Pick a slot");
        assert_eq!(told.tone, CallTone::Blocked);
    }

    #[test]
    fn several_arrivals_speak_in_the_first_and_count_the_rest() {
        let told = told_of(&[
            arrival("a", "First", "one", CallTone::Waiting),
            arrival("b", "Second", "two", CallTone::Waiting),
            arrival("c", "Third", "three", CallTone::Waiting),
        ])
        .expect("a banner");
        assert_eq!(told.session_id, "a");
        assert_eq!(told.title, "First");
        assert_eq!(told.body, "one\nand 2 more");
    }

    #[test]
    fn the_tone_is_blocked_if_any_arrival_blocks() {
        let told = told_of(&[arrival("a", "A", "x", CallTone::Waiting), arrival("b", "B", "y", CallTone::Blocked)]).expect("a banner");
        assert_eq!(told.tone, CallTone::Blocked);
        let told = told_of(&[arrival("a", "A", "x", CallTone::Waiting), arrival("b", "B", "y", CallTone::Waiting)]).expect("a banner");
        assert_eq!(told.tone, CallTone::Waiting);
    }

    #[test]
    fn a_job_with_no_title_is_called_job() {
        assert_eq!(told_of(&[arrival("a", "", "x", CallTone::Waiting)]).expect("a banner").title, "Job");
    }

    #[test]
    fn what_a_job_snoozed_since_raised_is_dropped_before_the_banner() {
        let mut collector = Collector::default();
        collector.add(vec![arrival("a", "A", "x", CallTone::Waiting), arrival("b", "B", "y", CallTone::Blocked)]);
        collector.without(|job| job == "b");
        let told = collector.close().expect("a banner");
        assert_eq!((told.title.as_str(), told.tone), ("A", CallTone::Waiting), "the quiet one neither speaks nor sets the tone");
        collector.add(vec![arrival("b", "B", "y", CallTone::Blocked)]);
        collector.without(|job| job == "b");
        assert_eq!(collector.close(), None);
    }

    #[test]
    fn a_mac_waiting_to_pair_is_told_under_its_calls_key_and_opens_no_job() {
        let mut asked = arrival("mac:0a0b", "Confirm Work pairing", "Work wants to pair. Confirm if it shows 4821.", CallTone::Blocked);
        asked.kind = CallArrivalKind::MacPairing;
        asked.key = Some("mac:0a0b".into());
        let told = told_of(&[asked]).expect("a banner");
        assert_eq!(told.session_id, "mac:0a0b");
        assert_eq!(told.title, "Confirm Work pairing");
        assert!(is_call_key(&told.session_id));
        assert!(!is_call_key("s-20b4e00a"), "a Job's id opens the Job");
    }

    #[test]
    fn a_review_asked_of_him_is_told_under_its_calls_key_and_opens_no_job() {
        let key = "review:harbor/ledger-api#2231";
        let mut asked = arrival(key, "Say which stage of an invoice failed", "Review requested on harbor/ledger-api#2231, opened by mira", CallTone::Waiting);
        asked.kind = CallArrivalKind::ReviewRequested;
        asked.key = Some(key.into());
        // A reader that carries the key apart from `session_id` is told it all the same.
        asked.session_id = SessionId::carried("review");
        let told = told_of(&[asked]).expect("a banner");
        assert_eq!(told.session_id, key);
        assert_eq!(told.title, "Say which stage of an invoice failed");
        assert!(is_call_key(&told.session_id), "a review's banner opens the pull request, never a Job");
        assert!(is_review_key(&told.session_id));
        assert!(!is_review_key("mac:0a0b"));
    }

    #[test]
    fn nothing_arrived_tells_nothing() {
        assert_eq!(told_of(&[]), None);
        assert_eq!(Collector::default().close(), None);
    }

    #[test]
    fn the_window_opens_on_the_first_arrival_and_is_not_extended() {
        let mut collector = Collector::default();
        assert!(!collector.add(Vec::new()), "no news opens no window");
        assert!(collector.add(vec![arrival("a", "A", "x", CallTone::Waiting)]));
        assert!(!collector.add(vec![arrival("b", "B", "y", CallTone::Waiting)]), "a second arrival does not restart the clock");
        let told = collector.close().expect("a banner");
        assert_eq!(told.body, "x\nand 1 more");
        assert!(collector.add(vec![arrival("c", "C", "z", CallTone::Waiting)]), "the next arrival opens a new window");
        assert_eq!(collector.close().expect("a banner").title, "C");
    }

    #[test]
    fn each_tone_names_a_sound_the_native_app_ships() {
        let sounds = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sounds");
        for tone in [CallTone::Blocked, CallTone::Waiting] {
            assert!(sounds.join(sound_of(tone)).is_file(), "{} is not in crates/bridge-native/sounds", sound_of(tone));
        }
    }

    #[test]
    fn a_banner_opens_the_newest_ask_and_never_a_build_or_a_permission() {
        let jobs = crate::mock::jobs("job-view").expect("decodes");
        let mut one = jobs.iter().flat_map(|job| job.waiting_for.iter()).next().expect("a waiting item").clone();
        one.options.clear();
        one.artifact = None;
        let at = |id: &str, source: ipc::WaitingSource, since: &str| ipc::WaitingItem { id: id.into(), source, since: ipc::Instant::carried(since), ..one.clone() };
        let waiting = [
            at("sketch:Chip", ipc::WaitingSource::Sketch, "2026-11-20T10:00:00Z"),
            at("ask:call-1", ipc::WaitingSource::AskCard, "2026-11-20T11:00:00Z"),
            at("try:branch@tip", ipc::WaitingSource::ReadyToTry, "2026-11-20T12:00:00Z"),
            at("agent-note", ipc::WaitingSource::Agent, "2026-11-20T13:00:00Z"),
        ];
        assert_eq!(newest_ask(&waiting).as_deref(), Some("ask:call-1"), "a build to try and an agent's note with no choices are not asks");
        assert_eq!(newest_ask(&waiting[2..]), None);
    }

    #[test]
    fn a_banner_opens_the_ask_it_told_while_that_still_waits() {
        let jobs = crate::mock::jobs("job-view").expect("decodes");
        let mut one = jobs.iter().flat_map(|job| job.waiting_for.iter()).next().expect("a waiting item").clone();
        one.options.clear();
        one.artifact = None;
        let at = |id: &str, source: ipc::WaitingSource, since: &str| ipc::WaitingItem { id: id.into(), source, since: ipc::Instant::carried(since), ..one.clone() };
        let waiting = [
            at("sketch:Chip", ipc::WaitingSource::Sketch, "2026-11-20T10:00:00Z"),
            at("ask:call-1", ipc::WaitingSource::AskCard, "2026-11-20T11:00:00Z"),
            at("try:branch@tip", ipc::WaitingSource::ReadyToTry, "2026-11-20T12:00:00Z"),
        ];
        assert_eq!(ask_to_open(&waiting, Some("sketch:Chip")).as_deref(), Some("sketch:Chip"), "the ask the banner told, though another is newer");
        assert_eq!(ask_to_open(&waiting, Some("plan:3")).as_deref(), Some("ask:call-1"), "one answered since opens the newest");
        assert_eq!(ask_to_open(&waiting, Some("try:branch@tip")).as_deref(), Some("ask:call-1"), "a build to try is not the sheet's");
        assert_eq!(ask_to_open(&waiting, None).as_deref(), Some("ask:call-1"));
    }

    #[test]
    fn an_arrival_on_a_jobs_list_carries_its_item_to_the_banner_but_not_its_tag() {
        let mut asked = arrival("a", "Fix the build", "Pick a slot", CallTone::Blocked);
        asked.item = Some("sketch:Chip".into());
        let told = told_of(&[asked]).expect("a banner");
        assert_eq!((told.session_id.as_str(), told.item.as_deref()), ("a", Some("sketch:Chip")), "a Job's newer banner still replaces its older one");
    }
}
