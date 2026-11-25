//! The toasts drawn: the stack in the window's bottom trailing corner, the history he opens from the
//! bell or ⌥⌘N, and the bell itself in the title bar's end group, with a dot while a toast is unseen.
//!
//! **A press on a toast is the page's own press.** The pick opens the item on the Operator's page and
//! says Yes there, Later puts it last there, so the page and the toast never disagree on what is
//! answered. A toast ends when Your turn no longer holds its item, wherever it was answered.
//!
//! **An urgent toast also posts a banner**, with the blocked call's sound, when no window of the app is
//! in front and `features.notifications` is on: the toast alone would wait unseen behind another app.
//!
//! **History is kept on this Mac**, beside the deck's columns, and only from the owner's Fleet: a walk
//! leaves it as it was.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    div, prelude::*, AnyElement, AnyWindowHandle, App, Context, Entity, FocusHandle, FontWeight, Global, Hsla, KeyDownEvent, Role, SharedString,
    Stateful, Subscription, Task, Toggled, WeakEntity, Window,
};
use ipc::operator_board::{DispatchWay, OperatorDispatched};
use ipc::operator_page::{OperatorAnswer, OperatorTurnItem, OperatorTurnKind};
use ipc::JobState;
use ipc::{AgentSignIn, CallTone};

use super::{Ended, FleetNotice, NoticeAct, Shelf, Toast, Urgency};
use crate::keymap::Act;
use crate::labels::labelled;
use crate::log::Note;
use crate::look::{token, ActiveLook, Look};
use crate::operator::face;
use crate::operator::page::{Arrived, OperatorPageView};
use crate::operator::screen::{Front, OperatorScreen};
use crate::operator::view::{kind_chip, press, pressed, Weight};
use crate::refusal::{Refusal, Report};
use crate::reviews::model::{ReviewAsk, ReviewTool};
use crate::titlebar::parts::{glyph, icon_button, sans, tip};

/// A toast's width: its three presses on one line, keys and all.
const WIDTH: f32 = 400.;
/// How many cards peek out under the newest one in the pile.
const PEEK_DEPTH: usize = 2;

/// The most toasts the history file keeps.
const KEPT: usize = 200;

/// The bell, for the title bar to draw in its end group.
pub struct BellSlot(pub gpui::AnyView);

impl Global for BellSlot {}

/// The toasts of the window in front, for the keys that work from anywhere.
struct Live(WeakEntity<Toasts>, AnyWindowHandle);

impl Global for Live {}

/// Whether he is reading an artifact with everything else faded away: what the Operator raised waits for him to finish.
#[derive(Default)]
struct Reading(bool);

impl Global for Reading {}

/// He began or finished reading in the focus view. The Operator's toasts and its rulings stay held, and draw when he is done;
/// Fleet's states and the presses Fleet did not take show all the same.
pub fn reading(on: bool, cx: &mut App) {
    if cx.try_global::<Reading>().map_or(on, |now| now.0 != on) {
        cx.set_global(Reading(on));
        if let Some(toasts) = cx.try_global::<Live>().map(|live| live.0.clone()) {
            cx.defer(move |cx| {
                if let Some(toasts) = toasts.upgrade() {
                    toasts.update(cx, |_, cx| cx.notify());
                }
            });
        }
    }
}

/// Fleet's states as the title bar last read them, drawn as urgent toasts above the Operator's.
#[derive(Default)]
struct FleetNotices(Vec<FleetNotice>);

impl Global for FleetNotices {}

/// The words of the agents' signed-out toast, and of its banner when no window is in front.
const SIGNED_OUT_NEXT: &str = "Jobs stop at their next reply until it signs in again.";

/// The account as the toast names it: Fleet's word for it, or the agent where Fleet does not say.
fn account_said(account: &str) -> &str {
    if account.is_empty() {
        "The agent"
    } else {
        account
    }
}

/// The toast's and the banner's headline: the account, with its name as Fleet sends it, is signed out.
fn signed_out_said(account: &str) -> String {
    format!("{} is signed out", account_said(account))
}
const SIGNING_IN_NEXT: &str = "Finish signing in in your browser.";

/// Whether `now` begins a signed-out spell `was` did not hold. **The state found on connecting is not
/// news** (`was` is `None`), as Classic's `AgentSignInNotice` seeds it: the toast stands, the banner does not post.
fn signed_out_news(was: Option<&AgentSignIn>, now: &AgentSignIn) -> bool {
    match (was, &now.signed_out_since) {
        (Some(was), Some(since)) => was.signed_out_since.as_ref() != Some(since),
        _ => false,
    }
}

/// The agents' sign-in as Fleet last said, drawn as a held state beside Fleet's own. `None` until Fleet has
/// answered.
#[derive(Default)]
struct SignedOut(Option<AgentSignIn>);

impl Global for SignedOut {}

/// The toast while the agents' account is signed out: no close, red edge, and Sign in or its ring. It goes when the agent
/// answers again (`session_host::signed_out`).
fn signed_out_notice(cx: &App) -> Option<FleetNotice> {
    let state = cx.try_global::<SignedOut>().and_then(|kept| kept.0.clone()).filter(|state| state.signed_out_since.is_some())?;
    Some(FleetNotice {
        id: "agent-sign-in",
        headline: signed_out_said(&state.account).into(),
        next: if state.signing_in { SIGNING_IN_NEXT } else { SIGNED_OUT_NEXT }.into(),
        code: "",
        fault: true,
        act: Some(NoticeAct::SignIn { running: state.signing_in, account: state.account.into() }),
    })
}

/// A held state's press: Sign in, or the ring while the browser is open.
fn notice_act(act: NoticeAct, look: &Look) -> gpui::AnyElement {
    use gpui::AnimationExt as _;
    match act {
        NoticeAct::SignIn { running: true, .. } => labelled(
            div()
                .id("toast-signing-in-row")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .child(glyph("loader-circle", look.length(token::SPACE_3), look.color(token::FG_MUTED)).with_animation(
                    "toast-signing-in",
                    gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat(),
                    |mark, at| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(at))),
                ))
                .child("Signing in"),
            Role::Status,
            "Signing in",
        )
        .into_any_element(),
        NoticeAct::SignIn { running: false, account } => labelled(
            press("toast-sign-in", "Sign in".into(), Some("user"), None, Weight::JobPick, look)
                .tooltip(tip(format!("Opens {}'s sign-in in your browser", if account.is_empty() { "the agent" } else { account.as_ref() })))
                .on_click(|_, _, cx| sign_in_pressed(cx)),
            Role::Button,
            "Sign in",
        )
        .into_any_element(),
    }
}

/// Sign in: Fleet runs the agent's own sign-in, which opens his browser, as Classic's press does. Fleet's
/// `agent_sign_in.changed` draws the ring. With no Fleet (a mock) the ring is drawn here.
fn sign_in_pressed(cx: &mut App) {
    let Some(toasts) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) else { return };
    match toasts.read(cx).page.read(cx).fleet().cloned() {
        Some(fleet) => crate::jobs::send_bare(fleet.read(cx).held().port(), "/agent/sign_in", cx),
        None => {
            let running = cx.try_global::<SignedOut>().and_then(|kept| kept.0.clone()).map(|state| AgentSignIn { signing_in: true, ..state });
            cx.set_global(SignedOut(running));
            toasts.update(cx, |_, cx| cx.notify());
        }
    }
}

/// What a told toast's Retry does again.
pub type Retry = Rc<dyn Fn(&mut App)>;

/// A sentence the app was told to raise, with its key, and Retry where the press can be sent again.
#[derive(Clone)]
struct TellOne {
    key: u64,
    sentence: SharedString,
    retry: Option<Retry>,
    /// A call Fleet refused: the facts under the headline, Copy debug info and Dispatch job.
    failure: Option<Failure>,
}

/// What a failure toast holds of a refusal, read when it was raised.
#[derive(Clone)]
struct Failure {
    refusal: Refusal,
    build: Option<ipc::FleetBuildReport>,
    report: Report,
    /// Dispatch job was pressed and Fleet has not answered.
    starting: bool,
}

/// Sentences the app was told to raise, oldest first: presses Fleet did not take.
#[derive(Default)]
struct Told(Vec<TellOne>);

impl Global for Told {}

/// What a press on a raised toast opens.
#[derive(Clone)]
enum Opens {
    /// The Job the Operator started or folded into, or a button on a Job's page started.
    Job(String),
    Operator,
    /// A pull request asked of him by name, in Reviews, under its call's key, and how far a review started from its
    /// toast has got.
    Review { key: String, asked: ipc::ReviewAsked, stage: Stage },
    /// A review started from a toast has ended: the pull request in Reviews, on Review comments.
    Ready { asked: ipc::ReviewAsked },
}

/// A review asked of him, as its toast stands.
#[derive(Clone)]
enum Stage {
    Asked,
    /// Review… pressed: the repository's tools once read, else why they were not, and what is ticked.
    Picking { tools: Option<Vec<ReviewTool>>, ask: ReviewAsk, why: Option<SharedString> },
    /// Start review pressed: one run a ticked tool, until each ends. Closed, it is hidden and still raises the ready toast.
    Running { tools: Vec<ReviewTool>, runs: Vec<Started>, hidden: bool },
}

/// One tool's run started from a toast: the head Fleet ran it at once known, and how it ended.
#[derive(Clone)]
struct Started {
    tool: String,
    head: Option<String>,
    ended: Option<RunEnded>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RunEnded {
    Found(usize),
    Failed,
}

/// How a run ended, where it has.
fn run_ended(run: &ipc::ReviewRun) -> Option<RunEnded> {
    match run.state {
        ipc::ReviewRunState::Done => Some(RunEnded::Found(match &run.result {
            Some(ipc::ReviewResult::Findings { findings }) => findings.len(),
            _ => 0,
        })),
        ipc::ReviewRunState::Failed => Some(RunEnded::Failed),
        ipc::ReviewRunState::Queued | ipc::ReviewRunState::Running => None,
    }
}

/// A run Fleet told of lands on the toast's run of its tool, where it is one: a whole-pull-request tool run, at the head
/// that run was started at once Fleet has said it.
fn land_review_run(asked: &ipc::ReviewAsked, runs: &mut [Started], run: &ipc::ReviewRun) {
    if run.manifest_id != asked.manifest_id || run.number != asked.number || run.kind != ipc::ReviewRunKind::Tool || run.since.is_some() {
        return;
    }
    if let Some(one) = runs.iter_mut().find(|one| one.tool == run.name && one.ended.is_none() && one.head.as_ref().is_none_or(|head| *head == run.head)) {
        one.head = Some(run.head.clone());
        one.ended = run_ended(run);
    }
}

/// `a`, `a and b`, `a, b and c`.
fn joined(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn capitalised(words: &str) -> String {
    let mut chars = words.chars();
    chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

/// The tools of these runs, as a sentence says them after another word.
fn tools_said<'a>(tools: &[ReviewTool], runs: impl Iterator<Item = &'a Started>) -> String {
    let named: Vec<String> = runs.map(|one| tools.iter().find(|tool| tool.id == one.tool).map_or_else(|| one.tool.clone(), |tool| tool.said.clone())).collect();
    joined(&named)
}

/// The running toast's line: `Possible bugs and risks on #2231. You'll get a notification when it's ready.`
fn running_said(asked: &ipc::ReviewAsked, tools: &[ReviewTool], runs: &[Started]) -> String {
    format!("{} on #{}. You'll get a notification when it's ready.", capitalised(&tools_said(tools, runs.iter())), asked.number)
}

/// The ready toast's line: what the tools found, then any that failed. `3 findings from possible bugs. Risks failed.`
fn ready_said(tools: &[ReviewTool], runs: &[Started]) -> String {
    let found: Vec<&Started> = runs.iter().filter(|one| matches!(one.ended, Some(RunEnded::Found(_)))).collect();
    let failed: Vec<&Started> = runs.iter().filter(|one| one.ended == Some(RunEnded::Failed)).collect();
    let mut said = Vec::new();
    if !found.is_empty() {
        let count: usize = found.iter().map(|one| if let Some(RunEnded::Found(count)) = one.ended { count } else { 0 }).sum();
        let from = tools_said(tools, found.iter().copied());
        said.push(match count {
            0 => format!("No findings from {from}."),
            1 => format!("1 finding from {from}."),
            many => format!("{many} findings from {from}."),
        });
    }
    if !failed.is_empty() {
        said.push(format!("{} failed.", capitalised(&tools_said(tools, failed.iter().copied()))));
    }
    said.join(" ")
}

/// A ready review's banner is tagged with this before its call's key, so a press on it opens Review comments.
pub const READY_TAG: &str = "review-ready:";

/// The pull requests of the ready reviews' banners, by tag, for a press on one.
#[derive(Default)]
struct ReadyBanners(Vec<(String, ipc::ReviewAsked)>);

impl Global for ReadyBanners {}

/// The pull request a ready review's banner names, while this run of the app holds it.
pub fn ready_asked(tag: &str, cx: &App) -> Option<ipc::ReviewAsked> {
    cx.try_global::<ReadyBanners>()?.0.iter().find(|(held, _)| held == tag).map(|(_, asked)| asked.clone())
}

/// A review toast as it stands: its call's key, the pull request and the stage.
fn review_of(key: u64, cx: &App) -> Option<(String, ipc::ReviewAsked, Stage)> {
    cx.try_global::<Rulings>()?.0.iter().find(|one| one.key == key).and_then(|one| match &one.opens {
        Opens::Review { key, asked, stage } => Some((key.clone(), asked.clone(), stage.clone())),
        _ => None,
    })
}

/// Change a review toast that still stands. The caller draws again.
fn restage(key: u64, cx: &mut App, change: impl FnOnce(&mut Raised)) {
    let mut rulings = cx.try_global::<Rulings>().map(|kept| kept.0.clone()).unwrap_or_default();
    let Some(one) = rulings.iter_mut().find(|one| one.key == key && matches!(one.opens, Opens::Review { .. })) else { return };
    change(one);
    cx.set_global(Rulings(rulings));
}

/// The stage of a review toast, to change in place.
fn stage_of(one: &mut Raised) -> Option<(&ipc::ReviewAsked, &mut Stage)> {
    match &mut one.opens {
        Opens::Review { asked, stage, .. } => Some((&*asked, stage)),
        _ => None,
    }
}

/// A test's stand-in for Fleet's review routes: the tools it lists, and each run asked of it.
#[cfg(test)]
#[derive(Default)]
struct FakeReviews {
    tools: Vec<ReviewTool>,
    posted: Vec<(String, ipc::StartReviewRun)>,
}

#[cfg(test)]
impl Global for FakeReviews {}

/// What the Operator did with a dispatch, or a review asked of him, as a toast of its own: its state, the sentence it
/// says, and what a press on it opens.
#[derive(Clone)]
struct Raised {
    key: u64,
    state: JobState,
    opens: Opens,
    sentence: SharedString,
    /// Fleet's line under the sentence, where it sends one.
    detail: Option<SharedString>,
}

impl Raised {
    /// The Operator's own: held while its page is in front, since the page holds each item.
    fn operators(&self) -> bool {
        !matches!(self.opens, Opens::Review { .. } | Opens::Ready { .. })
    }

    /// A running review he closed: it raises the ready toast all the same.
    fn hidden(&self) -> bool {
        matches!(self.opens, Opens::Review { stage: Stage::Running { hidden: true, .. }, .. })
    }
}

/// The Operator's rulings on dispatches, oldest first, each until he presses it or closes it.
#[derive(Default)]
struct Rulings(Vec<Raised>);

impl Global for Rulings {}

/// The words of a dispatch the Operator holds, by its id, so a toast that asks can say what about.
#[derive(Default)]
struct Holding(Vec<(String, String)>);

impl Global for Holding {}

/// A dispatch the Operator holds: what he sent, kept until `operator.dispatched` says what became of it.
pub fn dispatch_held(dispatch_id: &str, words: &str, cx: &mut App) {
    let mut held = cx.try_global::<Holding>().map(|kept| kept.0.clone()).unwrap_or_default();
    held.retain(|(id, _)| id != dispatch_id);
    held.push((dispatch_id.to_string(), words.to_string()));
    cx.set_global(Holding(held));
}

/// The words a toast names a dispatch by: a line, cut where it runs long.
pub(crate) fn about(words: &str) -> String {
    let line = words.lines().next().unwrap_or_default().trim();
    if line.chars().count() > 48 {
        format!("{}...", line.chars().take(48).collect::<String>().trim_end())
    } else {
        line.to_string()
    }
}

/// The Operator did something with a dispatch: a toast says it, and a press on it opens the Job or the Operator's
/// page. He stays where he was until he presses.
pub fn dispatched(news: OperatorDispatched, cx: &mut App) {
    let held = cx.try_global::<Holding>().and_then(|kept| kept.0.iter().find(|(id, _)| *id == news.dispatch_id).map(|(_, words)| about(words)));
    let sentence = ruling_said(&news, held);
    let opens = news.session_id.as_ref().map(|id| id.as_str().to_string()).filter(|_| matches!(news.way, DispatchWay::Started | DispatchWay::Handed)).map_or(Opens::Operator, Opens::Job);
    // The glyph is the registry's for the state the work stands in: working, waiting on him, or held as an idea.
    let state = match news.way {
        DispatchWay::Started | DispatchWay::Handed => JobState::Working,
        DispatchWay::Asked => JobState::Waiting,
        DispatchWay::Parked => JobState::Held,
    };
    raise(state, opens, sentence.into(), None, cx);
}

fn raise(state: JobState, opens: Opens, sentence: SharedString, detail: Option<SharedString>, cx: &mut App) {
    let mut rulings = cx.try_global::<Rulings>().map(|kept| kept.0.clone()).unwrap_or_default();
    let key = rulings.last().map_or(1, |one| one.key + 1);
    rulings.push(Raised { key, state, opens, sentence, detail });
    cx.set_global(Rulings(rulings));
    ring(cx);
}

/// A button on a Job's page started a Job: a toast says it, as the Operator's start does, and a press on it opens the Job.
pub fn job_started(job: &str, sentence: impl Into<SharedString>, cx: &mut App) {
    raise(JobState::Working, Opens::Job(job.to_string()), sentence.into(), None, cx);
}

/// Somebody asked him by name to review a pull request: a toast names it with Fleet's line under it, and a press on it
/// opens it in Reviews. One toast a request, which goes once he has opened it anywhere.
pub fn review_asked(key: &str, asked: ipc::ReviewAsked, said: &str, cx: &mut App) {
    let standing = cx.try_global::<Rulings>().is_some_and(|kept| kept.0.iter().any(|one| matches!(&one.opens, Opens::Review { key: held, .. } if held == key)));
    if standing {
        return;
    }
    let sentence = SharedString::from(asked.title.clone());
    raise(JobState::Waiting, Opens::Review { key: key.to_string(), asked, stage: Stage::Asked }, sentence, Some(said.to_string().into()), cx);
}

/// Fleet's calls moved: a review's toast goes once he has opened that pull request, here or anywhere else. One with
/// its picker open or a review running stays, as he started something on it.
fn settle_reviews(settled: impl Fn(&str) -> bool, cx: &mut App) -> bool {
    let Some(kept) = cx.try_global::<Rulings>() else { return false };
    let left: Vec<Raised> = kept.0.iter().filter(|one| !matches!(&one.opens, Opens::Review { key, stage: Stage::Asked, .. } if settled(key))).cloned().collect();
    if left.len() == kept.0.len() {
        return false;
    }
    cx.set_global(Rulings(left));
    true
}

/// The bell says a ruling is new. Deferred, because a caller may be inside the toasts' own update, as their
/// `Dispatched` subscriber is, and updating them again there is gpui's double lease panic (14 Nov crash).
fn ring(cx: &mut App) {
    let Some(toasts) = cx.try_global::<Live>().map(|live| live.0.clone()) else { return };
    cx.defer(move |cx| {
        if let Some(toasts) = toasts.upgrade() {
            toasts.update(cx, |toasts, cx| {
                toasts.unseen = true;
                cx.notify();
            });
        }
    });
}

/// What the Operator did, in bare words: the Job by its title, and for a question the title or what he sent.
fn ruling_said(news: &OperatorDispatched, held: Option<String>) -> String {
    match news.way {
        DispatchWay::Started => format!("Operator started {}", news.title.as_deref().unwrap_or("a Job")),
        DispatchWay::Handed => format!("Operator handed it to {}", news.title.as_deref().unwrap_or("a live Job")),
        DispatchWay::Asked => format!("Operator asks about {}", news.title.clone().or(held).unwrap_or_else(|| "it".into())),
        DispatchWay::Parked => "Operator kept it as an idea".to_string(),
    }
}

/// A ruling's toast went: pressed or closed.
fn unrule(key: u64, cx: &mut App) {
    let rulings = cx.try_global::<Rulings>().map(|kept| kept.0.iter().filter(|one| one.key != key).cloned().collect()).unwrap_or_default();
    cx.set_global(Rulings(rulings));
}

/// A failure toast that says one sentence, as Electron's `askToTell`: a press Fleet did not take, such as
/// a message to a Job. It stays until he closes it.
pub fn tell(sentence: impl Into<SharedString>, cx: &mut App) {
    told(sentence.into(), None, None, cx);
}

/// A failure toast with Retry, which sends the press again: a reply from Fleet that never arrived.
pub fn tell_retry(sentence: impl Into<SharedString>, retry: Retry, cx: &mut App) {
    told(sentence.into(), Some(retry), None, cx);
}

/// A call Fleet refused, as Classic's failure toast says it: `what` failed and why, Fleet's own sentence, the facts of
/// the call and of the build, then Dismiss, Copy debug info and Dispatch job. Fleet's debug report is read now, for
/// the Armada and harness rows and for what Copy and Dispatch job carry.
pub fn tell_refused(what: impl Into<String>, refusal: Refusal, cx: &mut App) {
    let fleet = live_fleet(cx);
    let held = fleet.as_ref().map(|fleet| fleet.read(cx).held());
    let build = held.and_then(|held| held.build.clone());
    let port = held.and_then(|held| held.port());
    let headline = format!("{}: {}", what.into(), refusal.short());
    let report = if port.is_some() { Report::Reading } else { Report::Unread("Fleet is not connected".into()) };
    let key = told(headline.into(), None, Some(Failure { refusal, build, report, starting: false }), cx);
    let Some(port) = port else { return };
    let read = cx.background_spawn(async move { crate::refusal::read_report(port) });
    cx.spawn(async move |cx| {
        let report = read.await;
        let _ = cx.update(|cx| change_failure(key, |failure| failure.report = report, cx));
    })
    .detach();
}

/// The walk's refused send: a dispatch with three pictures Fleet refused as too large, on a build four behind main,
/// with its debug report read, so every row and button draws as on the live app.
pub fn refused_too_large(cx: &mut App) {
    const SAID: &[u8] = br#"{"code":"request.too_large","message":"POST /sessions/dispatch carried 14.2 MB; Fleet takes at most 10 MB.","run_id":"walk","fields":{},"chain":[]}"#;
    let refusal = Refusal::read("POST", "/sessions/dispatch", 413, SAID).sent(14_900_000).pictures(3);
    let build = ipc::decode::<ipc::FleetBuildReport>("FleetBuildReport", br#"{"on":"main","commit":"9f3c2a1b7d0e4c5a","position":{"ahead":0,"behind":4}}"#).ok();
    let report = Report::Read("Armada debug report\n\nInstall\n  armada            0.4.2\n  runs from         ~/bin/armada\n  armada resolves   yes\n  agent             2.1.3\n".into());
    let headline = format!("The Job was not dispatched: {}", refusal.short());
    told(headline.into(), None, Some(Failure { refusal, build, report, starting: false }), cx);
}

/// Copy debug info: the headline, Fleet's sentence, the rows and the debug report, on the clipboard.
fn copy_failure(key: u64, cx: &mut App) {
    let Some(one) = cx.try_global::<Told>().and_then(|kept| kept.0.iter().find(|one| one.key == key).cloned()) else { return };
    let Some(failure) = one.failure else { return };
    let text = crate::refusal::debug_text(&one.sentence, &failure.refusal, failure.build.as_ref(), &failure.report);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
}

/// The dispatch Dispatch job sends: a bug in Armada's own repository, its words the debug text, as its brief and its
/// first message.
fn failure_dispatch(headline: &str, failure: &Failure, repository: ipc::ManifestId) -> ipc::DispatchJob {
    let facts = crate::refusal::debug_text(headline, &failure.refusal, failure.build.as_ref(), &failure.report);
    let text = format!("Fleet refused a call the native Bridge made. Find why and fix it.\n\n{facts}");
    let start = ipc::StartSession {
        manifest_id: repository,
        title: None,
        model: None,
        effort: None,
        mode: None,
        fork: None,
        root: None,
        request: Some(text.clone()),
        held: false,
        after: None,
        project_rows: Vec::new(),
        on_your_words: None,
        project: None,
        workflow: None,
    };
    let then = vec![ipc::DispatchThen::Message(ipc::DispatchMessage { text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() })];
    ipc::DispatchJob { start, kind: Some(ipc::DispatchKind::Bug), then }
}

/// The Fleet the toasts read, where the window has one.
fn live_fleet(cx: &App) -> Option<Entity<crate::fleet::FleetConnection>> {
    let toasts = cx.try_global::<Live>()?.0.upgrade()?;
    toasts.read(cx).page.read(cx).fleet().cloned()
}

/// Change a failure toast that still stands, and draw it again.
fn change_failure(key: u64, change: impl FnOnce(&mut Failure), cx: &mut App) {
    let mut told = cx.try_global::<Told>().map(|kept| kept.0.clone()).unwrap_or_default();
    let Some(failure) = told.iter_mut().find(|one| one.key == key).and_then(|one| one.failure.as_mut()) else { return };
    change(failure);
    cx.set_global(Told(told));
    if let Some(toasts) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
        toasts.update(cx, |_, cx| cx.notify());
    }
}

fn told(sentence: SharedString, retry: Option<Retry>, failure: Option<Failure>, cx: &mut App) -> u64 {
    // While "Fleet is slow" stands, a request that timed out is that notice's news, not a toast of its own.
    let slow = cx.try_global::<FleetNotices>().is_some_and(|kept| kept.0.iter().any(|notice| notice.id == crate::titlebar::SLOW_ID));
    if slow && failure.is_none() && sentence.contains("no answer in ") {
        return 0;
    }
    let mut told = cx.try_global::<Told>().map(|kept| kept.0.clone()).unwrap_or_default();
    let key = told.last().map_or(1, |one| one.key + 1);
    told.push(TellOne { key, sentence, retry, failure });
    cx.set_global(Told(told));
    if let Some(toasts) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
        toasts.update(cx, |toasts, cx| {
            toasts.unseen = true;
            cx.notify();
        });
    }
    key
}

/// No Fleet state and nothing told: a walk's part starts on a screen built again, with no toast of the
/// part before.
pub fn at_rest(cx: &mut App) {
    cx.set_global(FleetNotices::default());
    cx.set_global(Told::default());
    cx.set_global(Rulings::default());
    cx.set_global(Holding::default());
}

/// He closed a told toast.
fn untell(key: u64, cx: &mut App) {
    let told = cx.try_global::<Told>().map(|kept| kept.0.iter().filter(|one| one.key != key).cloned().collect()).unwrap_or_default();
    cx.set_global(Told(told));
}

/// The title bar read Fleet's state again: what it says now stands, and a state not shown before marks
/// the bell.
pub fn fleet(notices: Vec<FleetNotice>, cx: &mut App) {
    let was: Vec<&'static str> = cx.try_global::<FleetNotices>().map(|kept| kept.0.iter().map(|notice| notice.id).collect()).unwrap_or_default();
    let fresh = notices.iter().any(|notice| !was.contains(&notice.id));
    cx.set_global(FleetNotices(notices));
    if let Some(toasts) = cx.try_global::<Live>().and_then(|live| live.0.upgrade()) {
        toasts.update(cx, |toasts, cx| {
            toasts.unseen |= fresh;
            cx.notify();
        });
    }
}

/// ⌥⌘J and ⌥⌘N, from anywhere (`actions.toml`, scope `anywhere`).
pub fn bind(cx: &mut App) {
    cx.on_action(|key: &Act, cx: &mut App| {
        let history = match key.id {
            "toast_focus" => false,
            "toast_history" => true,
            _ => return cx.propagate(),
        };
        let Some(Live(toasts, handle)) = cx.try_global::<Live>().map(|live| Live(live.0.clone(), live.1)) else { return };
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                if let Some(toasts) = toasts.upgrade() {
                    toasts.update(cx, |toasts, cx| if history { toasts.toggle_history(window, cx) } else { toasts.focus_newest(window, cx) });
                }
            });
        });
    });
}

pub struct Toasts {
    shelf: Shelf,
    page: Entity<OperatorPageView>,
    screen: WeakEntity<OperatorScreen>,
    history: bool,
    /// A toast raised since he last opened the history.
    unseen: bool,
    /// The newest toast's keys: whichever toast is at the bottom tracks it.
    focus: FocusHandle,
    history_focus: FocusHandle,
    /// Where the keys were before ⌥⌘J or the history took them.
    back: Option<FocusHandle>,
    /// The history file, on the owner's Fleet only.
    file: Option<PathBuf>,
    /// How far the pile stands above its corner so it clears a message box drawn under it.
    lift: gpui::Pixels,
    /// The pointer is over the pile, so it fans out into a column.
    fanned: bool,
    /// The review toasts' asks of Fleet in flight, by what they ask.
    review_work: HashMap<String, Task<()>>,
    _subs: Vec<Subscription>,
}

impl Toasts {
    /// The toasts over the screen, and the bell in the title bar.
    pub fn mount(page: &Entity<OperatorPageView>, screen: WeakEntity<OperatorScreen>, window: &mut Window, cx: &mut App) -> Entity<Toasts> {
        let toasts = cx.new(|cx| Toasts::new(page.clone(), screen, window, cx));
        let bell = cx.new(|cx| Bell::new(&toasts, cx));
        cx.set_global(BellSlot(bell.into()));
        cx.set_global(Live(toasts.downgrade(), window.window_handle()));
        toasts
    }

    fn new(page: Entity<OperatorPageView>, screen: WeakEntity<OperatorScreen>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let file = page.read(cx).fleet().and(history_file());
        let shelf = Shelf::kept(file.as_ref().map(read_history).unwrap_or_default());
        let mut subs = vec![
            cx.subscribe_in(&page, window, |this, _, arrived: &Arrived, window, cx| this.arrived(&arrived.0, window, cx)),
            cx.observe(&page, |this, page, cx| this.page_moved(&page, cx)),
        ];
        // A mock scenario with no Fleet stands signed out, so the toast can be looked at.
        // Its state is a fixture, as Fleet would send it, so the account's name is not spelled here.
        let mock = crate::mock::scenario_is("agent-signed-out")
            .then(|| ipc::decode::<AgentSignIn>("AgentSignIn", include_bytes!("../../fixtures/agent-signed-out.json")).ok())
            .flatten();
        cx.set_global(SignedOut(mock));
        // What the Operator did with a dispatch is said as a toast, from the same Fleet the page reads.
        if let Some(fleet) = page.read(cx).fleet().cloned() {
            // The agents' sign-in, from the same Fleet: one account signs every Job in or out.
            cx.set_global(SignedOut(fleet.read(cx).held().agent_sign_in.clone()));
            subs.push(cx.observe_in(&fleet, window, |this, fleet, window, cx| this.sign_in_moved(&fleet, window, cx)));
            subs.push(Self::hear_dispatches(&fleet, cx));
            subs.extend(Self::hear_reviews(&fleet, cx));
        }
        Toasts { shelf, page, screen, history: false, unseen: false, focus: cx.focus_handle(), history_focus: cx.focus_handle(), back: None, file, lift: gpui::px(0.), fanned: false, review_work: HashMap::new(), _subs: subs }
    }

    fn hear_dispatches(fleet: &Entity<crate::fleet::FleetConnection>, cx: &mut Context<Self>) -> Subscription {
        cx.subscribe(fleet, |_, _, done: &crate::fleet::Dispatched, cx| {
            for one in &done.0 {
                dispatched(one.clone(), cx);
            }
        })
    }

    /// A review asked of him raises its toast; the toast goes once Fleet says he opened it.
    fn hear_reviews(fleet: &Entity<crate::fleet::FleetConnection>, cx: &mut Context<Self>) -> Vec<Subscription> {
        vec![
            cx.subscribe(fleet, |_, fleet, arrived: &crate::fleet::Arrived, cx| {
                for one in arrived.0.iter().filter(|one| one.kind == ipc::CallArrivalKind::ReviewRequested) {
                    let Some(key) = one.key.as_deref() else { continue };
                    let Some(asked) = fleet.read(cx).held().review_asked(key).cloned() else { continue };
                    review_asked(key, asked, &one.text, cx);
                }
            }),
            cx.observe(fleet, |_, fleet, cx| {
                let held = fleet.read(cx).held();
                let settled: Vec<String> = cx
                    .try_global::<Rulings>()
                    .map(|kept| kept.0.iter().filter_map(|one| match &one.opens { Opens::Review { key, .. } if held.call_settled(key) => Some(key.clone()), _ => None }).collect())
                    .unwrap_or_default();
                if !settled.is_empty() && settle_reviews(|key| settled.iter().any(|one| one == key), cx) {
                    cx.notify();
                }
            }),
            cx.subscribe(fleet, |this, _, moved: &crate::fleet::ReviewRunsMoved, cx| this.review_runs_moved(moved, cx)),
        ]
    }

    // -- a review started from its toast -----------------------------------------------------------

    fn review_port(&self, cx: &App) -> Result<u16, String> {
        self.page.read(cx).fleet().and_then(|fleet| fleet.read(cx).held().port()).ok_or_else(|| "Fleet is not connected".to_string())
    }

    fn ask_tools(&self, manifest: String, cx: &mut Context<Self>) -> Task<Result<Vec<ReviewTool>, String>> {
        #[cfg(test)]
        {
            if let Some(fake) = cx.try_global::<FakeReviews>() {
                return Task::ready(Ok(fake.tools.clone()));
            }
        }
        let port = self.review_port(cx);
        cx.background_spawn(async move { crate::reviews::read_tools(port?, &manifest) })
    }

    fn ask_run(&self, manifest: String, run: ipc::StartReviewRun, cx: &mut Context<Self>) -> Task<Result<ipc::ReviewRun, String>> {
        #[cfg(test)]
        {
            if cx.has_global::<FakeReviews>() {
                let queued = ipc::ReviewRun {
                    manifest_id: ipc::ManifestId::carried(&manifest),
                    number: run.number,
                    kind: run.kind,
                    name: run.name.clone(),
                    head: "f00d".into(),
                    since: None,
                    state: ipc::ReviewRunState::Queued,
                    started_at: "2026-11-19T09:00:00.000Z".into(),
                    finished_at: None,
                    failed: None,
                    result: None,
                    cost_micros: None,
                };
                cx.global_mut::<FakeReviews>().posted.push((manifest, run));
                return Task::ready(Ok(queued));
            }
        }
        let port = self.review_port(cx);
        cx.background_spawn(async move { crate::reviews::post_run(port?, &manifest, &run) })
    }

    /// Review…: the picker opens in the toast, ticked as he last asked on this repository once its tools are read.
    fn review_pick(&mut self, key: u64, cx: &mut Context<Self>) {
        let Some((_, asked, Stage::Asked)) = review_of(key, cx) else { return };
        restage(key, cx, |one| {
            if let Some((_, stage)) = stage_of(one) {
                *stage = Stage::Picking { tools: None, ask: ReviewAsk::first(&[]), why: None };
            }
        });
        let read = self.ask_tools(asked.manifest_id.as_str().to_string(), cx);
        let task = cx.spawn(async move |this, cx| {
            let answer = read.await;
            let _ = this.update(cx, |this, cx| this.review_tools_read(key, answer, cx));
        });
        self.review_work.insert(format!("tools:{key}"), task);
        cx.notify();
    }

    fn review_tools_read(&mut self, key: u64, answer: Result<Vec<ReviewTool>, String>, cx: &mut Context<Self>) {
        let Some((_, asked, Stage::Picking { .. })) = review_of(key, cx) else { return };
        let last = crate::reviews::last_ask(&asked.repository, cx);
        restage(key, cx, |one| {
            let Some((_, stage)) = stage_of(one) else { return };
            match answer {
                Ok(tools) => {
                    let mut ask = last.unwrap_or_else(|| ReviewAsk::first(&tools));
                    ask.tools.retain(|id| tools.iter().any(|tool| tool.id == *id));
                    *stage = Stage::Picking { tools: Some(tools), ask, why: None };
                }
                Err(why) => {
                    if let Stage::Picking { why: was, .. } = stage {
                        *was = Some(why.into());
                    }
                }
            }
        });
        cx.notify();
    }

    fn review_toggle(&mut self, key: u64, tool: &str, cx: &mut Context<Self>) {
        restage(key, cx, |one| {
            if let Some((_, Stage::Picking { tools: Some(tools), ask, .. })) = stage_of(one) {
                ask.toggle(tool, tools);
            }
        });
        cx.notify();
    }

    fn review_cancel(&mut self, key: u64, cx: &mut Context<Self>) {
        restage(key, cx, |one| {
            if let Some((_, stage)) = stage_of(one) {
                *stage = Stage::Asked;
            }
        });
        cx.notify();
    }

    /// Start review: the ask is kept for the repository, and one run a ticked tool is asked of Fleet over the whole pull
    /// request, again where one is kept at an older head. He stays where he is.
    fn review_start(&mut self, key: u64, cx: &mut Context<Self>) {
        let Some((_, asked, Stage::Picking { tools: Some(tools), ask, .. })) = review_of(key, cx) else { return };
        if ask.tools.is_empty() {
            return;
        }
        crate::reviews::keep_ask(asked.repository.clone(), ask.clone(), cx);
        let runs: Vec<Started> = ask.tools.iter().map(|tool| Started { tool: tool.clone(), head: None, ended: None }).collect();
        let detail = running_said(&asked, &tools, &runs);
        restage(key, cx, |one| {
            one.state = JobState::Working;
            one.sentence = "Review running in the background".into();
            one.detail = Some(detail.into());
            if let Some((_, stage)) = stage_of(one) {
                *stage = Stage::Running { tools, runs, hidden: false };
            }
        });
        for tool in ask.tools {
            let run = ipc::StartReviewRun { number: asked.number, kind: ipc::ReviewRunKind::Tool, name: tool.clone(), again: true, place: None, finding: None, since: None };
            let answer = self.ask_run(asked.manifest_id.as_str().to_string(), run, cx);
            let name = tool.clone();
            let task = cx.spawn(async move |this, cx| {
                let answer = answer.await;
                let _ = this.update(cx, |this, cx| this.review_run_answered(key, &name, answer, cx));
            });
            self.review_work.insert(format!("run:{key}:{tool}"), task);
        }
        cx.notify();
    }

    /// Fleet answered a run's start: the run as it stands, or a refusal, which ends that tool's run as failed.
    fn review_run_answered(&mut self, key: u64, tool: &str, answer: Result<ipc::ReviewRun, String>, cx: &mut Context<Self>) {
        restage(key, cx, |one| {
            let Some((_, Stage::Running { runs, .. })) = stage_of(one) else { return };
            let Some(started) = runs.iter_mut().find(|one| one.tool == tool && one.ended.is_none()) else { return };
            match answer {
                Ok(run) => {
                    started.ended = run_ended(&run);
                    started.head = Some(run.head);
                }
                Err(_) => started.ended = Some(RunEnded::Failed),
            }
        });
        self.review_ended(key, cx);
        cx.notify();
    }

    /// `review_run.changed`: each running review's runs land, and a gap reads theirs again.
    fn review_runs_moved(&mut self, moved: &crate::fleet::ReviewRunsMoved, cx: &mut Context<Self>) {
        let running: Vec<u64> = cx
            .try_global::<Rulings>()
            .map(|kept| kept.0.iter().filter(|one| matches!(one.opens, Opens::Review { stage: Stage::Running { .. }, .. })).map(|one| one.key).collect())
            .unwrap_or_default();
        if running.is_empty() {
            return;
        }
        for key in running {
            restage(key, cx, |one| {
                if let Some((asked, Stage::Running { runs, .. })) = stage_of(one) {
                    for run in &moved.runs {
                        land_review_run(asked, runs, run);
                    }
                }
            });
            self.review_ended(key, cx);
            if moved.lost {
                self.review_reread(key, cx);
            }
        }
        cx.notify();
    }

    fn review_reread(&mut self, key: u64, cx: &mut Context<Self>) {
        let Some((_, asked, Stage::Running { .. })) = review_of(key, cx) else { return };
        let Ok(port) = self.review_port(cx) else { return };
        let manifest = asked.manifest_id.as_str().to_string();
        let read = cx.background_spawn(async move { crate::reviews::read_runs(port, &manifest, asked.number) });
        let task = cx.spawn(async move |this, cx| {
            let Ok(runs) = read.await else { return };
            let _ = this.update(cx, |this, cx| this.review_runs_moved(&crate::fleet::ReviewRunsMoved { runs, lost: false }, cx));
        });
        self.review_work.insert(format!("reread:{key}"), task);
    }

    /// Every run ended: the running toast goes and the ready one is raised, with a banner when no window of the app is
    /// in front and `features.notifications` is on, as `notify` keeps.
    fn review_ended(&mut self, key: u64, cx: &mut Context<Self>) {
        let Some((call, asked, Stage::Running { tools, runs, .. })) = review_of(key, cx) else { return };
        if runs.iter().any(|one| one.ended.is_none()) {
            return;
        }
        unrule(key, cx);
        let sentence = format!("Review ready: {}", asked.title);
        let detail = ready_said(&tools, &runs);
        raise(JobState::Waiting, Opens::Ready { asked: asked.clone() }, sentence.clone().into(), Some(detail.clone().into()), cx);
        let notifications = self.page.read(cx).fleet().is_some_and(|fleet| fleet.read(cx).held().notifications);
        if notifications && cx.active_window().is_none() {
            let tag = format!("{READY_TAG}{call}");
            let banners = &mut cx.default_global::<ReadyBanners>().0;
            banners.retain(|(held, _)| *held != tag);
            banners.push((tag.clone(), asked));
            crate::banner::post(&tag, &sentence, &detail, crate::notify::sound_of(CallTone::Waiting));
        }
    }

    fn arrived(&mut self, items: &[OperatorTurnItem], window: &mut Window, cx: &mut Context<Self>) {
        let raised: Vec<OperatorTurnItem> = self.shelf.raise(items).into_iter().map(|toast| toast.item.clone()).collect();
        if raised.is_empty() {
            return;
        }
        self.unseen = true;
        let notifications = self.page.read(cx).fleet().is_some_and(|fleet| fleet.read(cx).held().notifications);
        if notifications && !window.is_window_active() {
            for item in raised.iter().filter(|item| item.urgency == Urgency::Urgent) {
                crate::banner::post(&item.id, &face::card_title(item), &sentence(item), crate::notify::sound_of(CallTone::Blocked));
            }
        }
        self.keep(cx);
    }

    /// Fleet's page moved: an open toast whose item has left Your turn was settled, here or anywhere.
    fn page_moved(&mut self, page: &Entity<OperatorPageView>, cx: &mut Context<Self>) {
        let page = page.read(cx);
        if !page.is_read() {
            return;
        }
        let waiting: Vec<String> = page.items().iter().map(|turn| turn.id.clone()).collect();
        let waiting: Vec<&str> = waiting.iter().map(String::as_str).collect();
        self.shelf.keep_open(&waiting);
        self.keep(cx);
    }

    /// Write the history, where it is kept, and draw again.
    fn keep(&mut self, cx: &mut Context<Self>) {
        if let Some(file) = &self.file {
            let mut lines: Vec<String> = self.shelf.history().take(KEPT).filter_map(line_of).collect();
            lines.reverse();
            if let Err(why) = std::fs::write(file, lines.join("\n")) {
                Note::warn("the toasts' history was not kept").field("why", why.to_string()).emit();
            }
        }
        cx.notify();
    }

    // -- presses -----------------------------------------------------------------------------------

    /// The pick: Yes to the item on the page, with the Operator's pick where it has one.
    fn pick(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| {
            page.open_id(&id, cx);
            page.answer(OperatorAnswer::Yes, window, cx);
        });
        self.ended(&id, Ended::Settled, window, cx);
    }

    fn later(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| {
            page.open_id(&id, cx);
            // A toast has no menu: its Later is the rest of today.
            page.later(None, cx);
        });
        self.ended(&id, Ended::Later, window, cx);
    }

    fn close(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.ended(&id, Ended::Closed, window, cx);
    }

    /// The Operator's page in front, the item open on it.
    fn open_operator(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.page.update(cx, |page, cx| page.open_id(&id, cx));
        self.history = false;
        self.back = None;
        if let Some(screen) = self.screen.upgrade() {
            screen.update(cx, |screen, cx| screen.bring(Some(Front::Operator), window, cx));
        }
        cx.notify();
    }

    /// A press on an Operator ruling's toast: the Job it started or took the work, or the Operator's page. The toast goes.
    fn open_ruling(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = cx.try_global::<Rulings>().and_then(|kept| kept.0.iter().find(|one| one.key == key).cloned()) else { return };
        // A running review's toast stays until the ready one takes its place.
        let running = matches!(one.opens, Opens::Review { stage: Stage::Running { .. }, .. });
        if !running {
            unrule(key, cx);
        }
        self.history = false;
        self.back = None;
        if let Some(screen) = self.screen.upgrade() {
            screen.update(cx, |screen, cx| match &one.opens {
                Opens::Job(job) => screen.open_job(job, window, cx),
                Opens::Operator => screen.bring(Some(Front::Operator), window, cx),
                Opens::Review { asked, stage: Stage::Running { .. }, .. } | Opens::Ready { asked } => screen.open_review_findings(asked, window, cx),
                Opens::Review { asked, .. } => screen.open_review(Some(asked), window, cx),
            });
        }
        cx.notify();
    }

    /// He closed a ruling's toast: the Job or the page stays unopened.
    fn close_ruling(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let had_keys = self.focus.is_focused(window);
        let running = review_of(key, cx).is_some_and(|(_, _, stage)| matches!(stage, Stage::Running { .. }));
        if running {
            restage(key, cx, |one| {
                if let Some((_, Stage::Running { hidden, .. })) = stage_of(one) {
                    *hidden = true;
                }
            });
        } else {
            unrule(key, cx);
        }
        if had_keys {
            self.give_back(window, cx);
        }
        cx.notify();
    }

    /// A toast went: the keys stay on the toasts while one is left, else go back where they were.
    fn ended(&mut self, id: &str, how: Ended, window: &mut Window, cx: &mut Context<Self>) {
        let had_keys = self.focus.is_focused(window);
        self.shelf.end(id, how);
        if had_keys && self.shelf.shown().is_empty() {
            self.give_back(window, cx);
        }
        self.keep(cx);
    }

    fn give_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.back.take() {
            window.focus(&back, cx);
        }
    }

    pub fn focus_newest(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ruled = cx.try_global::<Rulings>().is_some_and(|kept| !kept.0.is_empty());
        if self.shelf.shown().is_empty() && !ruled {
            return;
        }
        if !self.focus.is_focused(window) {
            self.back = window.focused(cx).filter(|was| was != &self.history_focus);
        }
        self.history = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub fn toggle_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.history = !self.history;
        if self.history {
            self.unseen = false;
            if !self.focus.is_focused(window) {
                self.back = window.focused(cx);
            }
            window.focus(&self.history_focus, cx);
        } else {
            self.give_back(window, cx);
        }
        cx.notify();
    }

    /// A, L and Esc on the newest toast.
    fn toast_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;
        if key.modifiers.modified() {
            return;
        }
        let Some(id) = self.shelf.shown().last().map(|toast| toast.item.id.clone()) else { return };
        match key.key.as_str() {
            "a" => self.pick(id, window, cx),
            "l" => self.later(id, window, cx),
            "escape" => self.close(id, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    // -- drawn -------------------------------------------------------------------------------------

    fn toast(&self, toast: &Toast, newest: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let item = &toast.item;
        let id = item.id.clone();
        let title = face::card_title(item);
        let pick_word: SharedString = match item.kind {
            OperatorTurnKind::Decision => face::pick_word(item, None),
            OperatorTurnKind::GoAhead if face::picks_later(item) => "Later".into(),
            OperatorTurnKind::GoAhead => "Yes".into(),
        };
        let pick_said = face::pick_sentence(item);
        let muted = muted_line(item);
        let key = |word: &'static str| newest.then(|| SharedString::from(word));
        let pick = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-pick:{id}")), pick_word.clone(), None, key("A"), Weight::Pick, look)
                    .when_some(pick_said, |button, said| button.tooltip(tip(said)))
                    .on_click(cx.listener(move |this, _, window, cx| this.pick(id.clone(), window, cx))),
                Role::Button,
                pick_word,
            )
        };
        let later = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-later:{id}")), "Later".into(), Some("clock"), key("L"), Weight::Bare, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.later(id.clone(), window, cx))),
                Role::Button,
                "Later",
            )
        };
        let open = {
            let id = id.clone();
            labelled(
                press(SharedString::from(format!("toast-open:{id}")), "Open the Operator".into(), None, None, Weight::Bare, look)
                    .on_click(cx.listener(move |this, _, window, cx| this.open_operator(id.clone(), window, cx))),
                Role::Button,
                "Open the Operator",
            )
        };
        let close = {
            let id = id.clone();
            labelled(
                icon_button(SharedString::from(format!("toast-close:{id}")), "x", look.color(token::FG_SUBTLE), look)
                    .tooltip(tip("Close — Esc"))
                    .on_click(cx.listener(move |this, _, window, cx| this.close(id.clone(), window, cx))),
                Role::Button,
                format!("Close toast: {title}"),
            )
        };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph("bot", look.length(token::SPACE_4), look.color(token::OPERATOR)))
            .child(
                div()
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(format!("Operator · {}", clock_of(item.raised_at.as_str()))),
            )
            .child(kind_chip(face::kind_of(item), look))
            .child(div().flex_1())
            .child(close);
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(head)
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(title.clone()))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).line_clamp(2).child(sentence(item)))
            .children(muted.map(|line| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).truncate().child(line)))
            .child(div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).child(pick).child(later).child(open));
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let frame = div()
            .id(SharedString::from(format!("toast:{id}")))
            .w(gpui::px(WIDTH))
            .flex()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge_of(toast.urgency, look))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            // A press on a toast never reaches what is under it.
            .occlude()
            .font_family(sans(look))
            .child(body);
        let frame = if newest {
            frame
                .track_focus(&self.focus)
                .focus(move |style| style.border_color(ring))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| this.toast_key(event, window, cx)))
        } else {
            frame
        };
        labelled(frame, Role::Alert, format!("Toast: {title}"))
    }

    /// A state of Fleet: its edge, what is wrong and what to do, and its code. It has no close: it goes
    /// when the state does. Copy debug info is on the Fleet popover.
    fn fleet_toast(&self, notice: &FleetNotice, look: &Look) -> impl IntoElement {
        let edge = look.color(if notice.fault { token::STATUS_COMPLETED_FAILED } else { token::DEGRADED });
        // The agents' sign-in is the agent's state, not Fleet's, and carries no code.
        let (mark, source): (&str, SharedString) = match &notice.act {
            Some(NoticeAct::SignIn { account, .. }) => ("bot", if account.is_empty() { "Agent".into() } else { account.clone() }),
            None => ("server-off", "Fleet".into()),
        };
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(glyph(mark, look.length(token::SPACE_4), edge))
            .child(div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(source))
            .when(!notice.code.is_empty(), |head| {
                head.child(
                    div()
                        .px(look.length(token::SPACE_2))
                        .rounded(look.length(token::RADIUS_MD))
                        .bg(edge)
                        .text_color(look.color(token::FG_INVERSE))
                        .font_family(crate::titlebar::parts::mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .child(notice.code),
                )
            });
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(head)
            .child(div().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(notice.headline.clone()))
            .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(notice.next.clone()))
            .children(notice.act.clone().map(|act| div().flex().justify_end().pt(look.length(token::SPACE_1)).child(notice_act(act, look))));
        labelled(
            div()
                .id(SharedString::from(format!("toast:{}", notice.id)))
                .w(gpui::px(WIDTH))
                .flex()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge)
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                // A press on a toast never reaches what is under it.
                .occlude()
                .font_family(sans(look))
                .child(body),
            Role::Alert,
            format!("Toast: {}", notice.headline),
        )
    }

    /// Fleet moved: where the agents' sign-in changed it is drawn, and a spell just begun marks the bell and,
    /// with no window in front, posts a banner, as an urgent toast does.
    fn sign_in_moved(&mut self, fleet: &Entity<crate::fleet::FleetConnection>, window: &mut Window, cx: &mut Context<Self>) {
        let held = fleet.read(cx).held();
        let Some(now) = held.agent_sign_in.clone() else { return };
        let was = cx.try_global::<SignedOut>().and_then(|kept| kept.0.clone());
        if was.as_ref() == Some(&now) {
            return;
        }
        if signed_out_news(was.as_ref(), &now) {
            self.unseen = true;
            if held.notifications && !window.is_window_active() {
                crate::banner::post("agent-sign-in", &signed_out_said(&now.account), SIGNED_OUT_NEXT, crate::notify::sound_of(CallTone::Blocked));
            }
        }
        cx.set_global(SignedOut(Some(now)));
        cx.notify();
    }

    /// A sentence the app was told: the failed edge, the sentence, Retry where it has one, and its close.
    fn told_toast(&self, one: &TellOne, look: &Look, cx: &Context<Self>) -> AnyElement {
        if let Some(failure) = &one.failure {
            return self.failure_toast(one.key, &one.sentence, failure, look, cx).into_any_element();
        }
        let (key, sentence) = (one.key, &one.sentence);
        let edge = look.color(token::STATUS_COMPLETED_FAILED);
        let retry = one.retry.clone().map(|retry| {
            let button = press(SharedString::from(format!("toast-told-retry:{key}")), "Retry".into(), Some("rotate-cw"), None, Weight::Pick, look).on_click(cx.listener(move |_, _, _, cx| {
                untell(key, cx);
                retry(cx);
                cx.notify();
            }));
            labelled(button, Role::Button, "Retry")
        });
        let close = labelled(
            icon_button(SharedString::from(format!("toast-told-close:{key}")), "x", look.color(token::FG_MUTED), look)
                .tooltip(tip("Close"))
                .on_click(cx.listener(move |_, _, _, cx| {
                    untell(key, cx);
                    cx.notify();
                })),
            Role::Button,
            "Close",
        );
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(div().pt(look.length(token::SPACE_1) / 2.).child(glyph("circle-x", look.length(token::SPACE_4), edge)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(div().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(sentence.clone()))
                    .children(retry.map(|retry| div().flex().child(retry))),
            )
            .child(close);
        labelled(
            div()
                .id(SharedString::from(format!("toast-told:{key}")))
                .w(gpui::px(WIDTH))
                .flex()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge)
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                // A press on a toast never reaches what is under it.
                .occlude()
                .font_family(sans(look))
                .child(body),
            Role::Alert,
            format!("Toast: {sentence}"),
        )
        .into_any_element()
    }

    /// A call Fleet refused (`tell_refused`): the failed glyph and the headline, Fleet's sentence, a row for each fact
    /// with its label dimmer than its value, then Dismiss, Copy debug info and Dispatch job at the trailing edge.
    fn failure_toast(&self, key: u64, headline: &SharedString, failure: &Failure, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let edge = look.color(token::STATUS_COMPLETED_FAILED);
        let rows = failure.refusal.rows(failure.build.as_ref(), &failure.report).into_iter().map(|(label, value)| {
            labelled(
                div()
                    .id(SharedString::from(format!("toast-failure-row:{key}:{label}")))
                    .flex()
                    .items_baseline()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_8)).text_color(look.color(token::FG_MUTED)).child(label))
                    .child(div().flex_1().min_w_0().text_color(look.color(token::FG_DEFAULT)).child(value.clone())),
                Role::Label,
                format!("{label}: {value}"),
            )
        });
        let copy = labelled(
            press(SharedString::from(format!("toast-failure-copy:{key}")), "Copy debug info".into(), None, None, Weight::Plain, look)
                .tooltip(tip("Copies the facts and Fleet's debug report"))
                .on_click(cx.listener(move |_, _, _, cx| copy_failure(key, cx))),
            Role::Button,
            "Copy debug info",
        );
        let dismiss = labelled(
            press(SharedString::from(format!("toast-failure-dismiss:{key}")), "Dismiss".into(), None, None, Weight::Bare, look).on_click(cx.listener(move |_, _, _, cx| {
                untell(key, cx);
                cx.notify();
            })),
            Role::Button,
            "Dismiss",
        );
        let dispatch_word: SharedString = if failure.starting { "Dispatching".into() } else { "Dispatch job".into() };
        let dispatch = labelled(
            press(SharedString::from(format!("toast-failure-dispatch:{key}")), dispatch_word.clone(), None, None, Weight::JobPick, look)
                .tooltip(tip("Starts a Job in Armada's own repository, told all of this"))
                .when(!failure.starting, |button| button.on_click(cx.listener(move |this, _, window, cx| this.dispatch_failure(key, window, cx)))),
            Role::Button,
            dispatch_word,
        );
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_none().pt(look.length(token::SPACE_1) / 2.).child(glyph("circle-x", look.length(token::SPACE_4), edge)))
                    .child(div().flex_1().min_w_0().text_size(look.length(token::TEXT_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(headline.clone())),
            )
            .children(failure.refusal.said.clone().map(|said| labelled(div().id(SharedString::from(format!("toast-failure-said:{key}"))).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(said.clone()), Role::Label, said)))
            .child(div().flex().flex_col().gap(look.length(token::SPACE_1) / 2.).children(rows))
            .child(div().flex().justify_end().items_center().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_1)).child(dismiss).child(copy).child(dispatch));
        labelled(
            div()
                .id(SharedString::from(format!("toast-failure:{key}")))
                .w(gpui::px(WIDTH))
                .flex()
                .overflow_hidden()
                .rounded(look.length(token::RADIUS_LG) * 1.5)
                .border(look.length(token::BORDER_WIDTH))
                .border_color(edge)
                .bg(look.color(token::BG_OVERLAY))
                .shadow(look.shadows(token::SHADOW_OVERLAY))
                // A press on a toast never reaches what is under it.
                .occlude()
                .font_family(sans(look))
                .child(body),
            Role::Alert,
            format!("Toast: {headline}"),
        )
    }

    /// Dispatch job: a Job in Armada's own repository, told the refusal and Fleet's debug report, as the crash panel
    /// starts one (`crash::fix_dispatch`). The Job opens and the toast goes; a start Fleet refuses is a toast of its own,
    /// and this one stays, as Classic's does.
    fn dispatch_failure(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = cx.try_global::<Told>().and_then(|kept| kept.0.iter().find(|one| one.key == key).cloned()) else { return };
        let Some(failure) = one.failure.clone() else { return };
        let not_started = |why: String, cx: &mut Context<Self>| cx.defer(move |cx| tell(format!("Job not started: {why}"), cx));
        let Some(fleet) = self.page.read(cx).fleet().cloned() else { return not_started("Fleet is not connected".into(), cx) };
        let held = fleet.read(cx).held();
        let (port, repositories) = (held.port(), held.repositories.clone().unwrap_or_default());
        let Some(port) = port else { return not_started("Fleet is not connected".into(), cx) };
        let Some(repository) = crate::crash::armada_repository(&crate::dispatch::live::served(&repositories)) else {
            return not_started("Fleet serves no repository named armada".into(), cx);
        };
        let job = failure_dispatch(&one.sentence, &failure, repository);
        let words = job.start.request.clone().unwrap_or_default();
        cx.defer(move |cx| change_failure(key, |failure| failure.starting = true, cx));
        let sent = cx.background_spawn(async move {
            let body = ipc::encode(&job).map_err(|why| format!("{why:?}"))?;
            let bytes = crate::refusal::post(port, "/sessions/dispatch", body)?;
            ipc::decode::<ipc::JobDispatched>("JobDispatched", &bytes).map_err(|why| why.to_string())
        });
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| match answer {
                Ok(done) => {
                    untell(key, cx);
                    match (done.session, done.operator.or(done.parked)) {
                        (Some(record), _) => {
                            if let Some(screen) = this.screen.upgrade() {
                                screen.update(cx, |screen, cx| screen.open_job(record.id.as_str(), window, cx));
                            }
                        }
                        (None, Some(held)) => dispatch_held(&held, &words, cx),
                        (None, None) => {}
                    }
                    cx.notify();
                }
                Err(why) => {
                    let why: String = why;
                    cx.defer(move |cx| {
                        change_failure(key, |failure| failure.starting = false, cx);
                        tell(format!("Job not started: {why}"), cx);
                    });
                }
            });
        })
        .detach();
    }

    /// What the Operator did with a dispatch: its state glyph with a tooltip, the sentence as the press that opens
    /// the Job or the Operator's page, and its close.
    fn ruling_toast(&self, one: &Raised, newest: bool, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let key = one.key;
        let (name, said) = crate::jobs::glyph_of(one.state, None);
        let ink = look.color(crate::jobs::hue_of(one.state));
        let opens: SharedString = match one.opens {
            Opens::Job(_) => "Opens the Job".into(),
            Opens::Operator => "Opens the Operator".into(),
            Opens::Review { stage: Stage::Running { .. }, .. } | Opens::Ready { .. } => "Opens it in Reviews, on Review comments".into(),
            Opens::Review { .. } => "Opens it in Reviews".into(),
        };
        let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
        let mark = labelled(
            div().id(SharedString::from(format!("toast-ruling-state:{key}"))).flex_none().pt(look.length(token::SPACE_1) / 2.).tooltip(tip(said.clone())).child(glyph(name, look.length(token::SPACE_4), ink)),
            Role::Image,
            said,
        );
        let press = labelled(
            div()
                .id(SharedString::from(format!("toast-ruling:{key}")))
                .flex_1()
                .min_w_0()
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(gpui::transparent_black())
                .cursor_pointer()
                .focusable()
                .tab_stop(true)
                .focus_visible(move |style| style.border_color(ring))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .tooltip(tip(opens))
                .on_click(cx.listener(move |this, _, window, cx| this.open_ruling(key, window, cx)))
                .child(one.sentence.clone())
                .children(one.detail.clone().map(|detail| div().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(detail))),
            Role::Button,
            one.sentence.clone(),
        );
        let close = labelled(
            icon_button(SharedString::from(format!("toast-ruling-close:{key}")), "x", look.color(token::FG_MUTED), look)
                .tooltip(tip("Close"))
                .on_click(cx.listener(move |this, _, window, cx| this.close_ruling(key, window, cx))),
            Role::Button,
            "Close",
        );
        let head = div().flex().items_start().gap(look.length(token::SPACE_2)).child(mark).child(press).child(close);
        let below = match &one.opens {
            Opens::Review { asked, stage: Stage::Asked, .. } => Some(self.review_button(key, asked, look, cx)),
            Opens::Review { asked, stage: Stage::Picking { tools, ask, why }, .. } => Some(self.review_picker(key, asked, tools.as_deref(), ask, why.as_ref(), look, cx)),
            _ => None,
        };
        let body = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .child(head)
            .children(below);
        let picking = matches!(one.opens, Opens::Review { stage: Stage::Picking { .. }, .. });
        let frame = div()
            .id(SharedString::from(format!("toast-ruling-frame:{key}")))
            .w(gpui::px(WIDTH))
            .flex()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(ink)
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            // A press on a toast never reaches what is under it.
            .occlude()
            .font_family(sans(look))
            .child(body);
        // ⌥⌘J puts the keys on the newest toast: Enter opens it, Esc closes it.
        let frame = if newest {
            frame.track_focus(&self.focus).focus(move |style| style.border_color(ring)).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let pressed = &event.keystroke;
                if pressed.modifiers.modified() {
                    return;
                }
                match pressed.key.as_str() {
                    "enter" if picking => this.review_start(key, cx),
                    "escape" if picking => this.review_cancel(key, cx),
                    "enter" => this.open_ruling(key, window, cx),
                    "escape" => this.close_ruling(key, window, cx),
                    _ => return,
                }
                cx.stop_propagation();
            }))
        } else {
            frame
        };
        labelled(frame, Role::Alert, format!("Toast: {}", one.sentence))
    }

    /// Review… under a review asked of him, at the trailing edge.
    fn review_button(&self, key: u64, asked: &ipc::ReviewAsked, look: &Look, cx: &Context<Self>) -> AnyElement {
        let button = press(SharedString::from(format!("toast-review:{key}")), "Review…".into(), Some("bot"), None, Weight::Plain, look).tooltip(tip(format!("Picks review tools and starts a review of #{}", asked.number)));
        div().flex().justify_end().child(labelled(pressed(button, cx, move |this, _, cx| this.review_pick(key, cx)), Role::Button, "Review…")).into_any_element()
    }

    /// The picker in the toast: a checkbox a review tool, then Cancel and Start review. Bones while the tools are read.
    #[allow(clippy::too_many_arguments)]
    fn review_picker(&self, key: u64, asked: &ipc::ReviewAsked, tools: Option<&[ReviewTool]>, ask: &ReviewAsk, why: Option<&SharedString>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let muted = |id: String, said: String| labelled(div().id(SharedString::from(id)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(said.clone()), Role::Label, said);
        let mut list = div().flex().flex_col();
        match (tools, why) {
            (_, Some(why)) => list = list.child(muted(format!("toast-review-why:{key}"), format!("The review tools were not read: {why}"))),
            (None, None) => {
                let bone = |at: usize| div().id(SharedString::from(format!("toast-review-bone:{key}:{at}"))).flex().items_center().h(look.length(token::H_CONTROL)).child(div().h(look.length(token::SPACE_3)).w(gpui::relative(0.4 + 0.2 * at as f32)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::BG_HOVER)));
                list = list.child(labelled(div().id(SharedString::from(format!("toast-review-reading:{key}"))).flex().flex_col().child(bone(0)).child(bone(1)), Role::Status, "Reading the review tools"));
            }
            (Some([]), None) => list = list.child(muted(format!("toast-review-none:{key}"), "No review tool is set up for this repository".into())),
            (Some(tools), None) => {
                for tool in tools {
                    let on = ask.has(&tool.id);
                    let ink = if on { look.color(token::ACCENT) } else { look.color(token::FG_MUTED) };
                    let ring = look.edge(token::FOCUS_RING).map_or(look.color(token::ACCENT), |(_, colour)| colour);
                    let id = tool.id.clone();
                    let row = div()
                        .id(SharedString::from(format!("toast-review-tool:{key}:{}", tool.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .h(look.length(token::H_CONTROL))
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(gpui::transparent_black())
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .focus_visible(move |style| style.border_color(ring))
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .aria_toggled(if on { Toggled::True } else { Toggled::False })
                        .child(glyph(if on { "square-check" } else { "square" }, look.length(token::SPACE_4), ink))
                        .child(SharedString::from(tool.name.clone()));
                    list = list.child(labelled(pressed(row, cx, move |this, _, cx| this.review_toggle(key, &id, cx)), Role::CheckBox, tool.name.clone()));
                }
            }
        }
        let cancel = labelled(
            pressed(press(SharedString::from(format!("toast-review-cancel:{key}")), "Cancel".into(), None, None, Weight::Bare, look).tooltip(tip("Closes the picker")), cx, move |this, _, cx| this.review_cancel(key, cx)),
            Role::Button,
            "Cancel",
        );
        let ready = tools.is_some_and(|tools| !tools.is_empty()) && !ask.tools.is_empty();
        let start = press(SharedString::from(format!("toast-review-start:{key}")), "Start review".into(), Some("bot"), None, Weight::JobPick, look)
            .tooltip(tip(if ready { format!("Starts the ticked tools on #{}", asked.number) } else { "Tick a tool to start a review".to_string() }));
        let start = if ready { pressed(start, cx, move |this, _, cx| this.review_start(key, cx)) } else { start.opacity(0.5).cursor_default().tab_stop(false) };
        let start = labelled(start, Role::Button, "Start review");
        labelled(
            div()
                .id(SharedString::from(format!("toast-review-picker:{key}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .pt(look.length(token::SPACE_2))
                .border_t(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_DEFAULT))
                .child(list)
                .child(div().flex().justify_end().items_center().gap(look.length(token::SPACE_2)).child(cancel).child(start)),
            Role::Group,
            "Review tools",
        )
        .into_any_element()
    }

    /// How far to stand above the corner: from the corner to the top of the message box, where one is drawn under
    /// the pile's width, with a gap; none where no box is under it.
    fn lift_over_box(&self, window: &Window, look: &Look, cx: &App) -> gpui::Pixels {
        let none = gpui::px(0.);
        let Some(book) = cx.try_global::<crate::labels::LabelBook>() else { return none };
        let Some(boxed) = book.find(window.window_handle().window_id(), Role::Group, "Message box").map(|drawn| drawn.shown) else { return none };
        let size = window.viewport_size();
        let (right, width) = (size.width - look.length(token::SPACE_6), gpui::px(WIDTH));
        let under = boxed.left() < right && boxed.right() > right - width;
        if !under || boxed.size.height <= gpui::px(0.) {
            return none;
        }
        let corner = look.length(token::SPACE_6);
        (size.height - boxed.top() + look.length(token::SPACE_3) - corner).max(none)
    }

    fn history_panel(&self, look: &Look, cx: &Context<Self>) -> impl IntoElement {
        let rows = self.shelf.history().map(|toast| {
            let id = toast.item.id.clone();
            let title = face::card_title(&toast.item);
            let said = match toast.ended {
                None => "Open",
                Some(Ended::Settled) => "Settled",
                Some(Ended::Closed) => "Closed",
                Some(Ended::Later) => "Later",
            };
            let row = div()
                .id(SharedString::from(format!("toast-history:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .h(look.length(token::H_CONTROL_LG))
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(div().flex_none().size(look.length(token::SPACE_2)).rounded_full().bg(edge_of(toast.urgency, look)))
                .child(kind_chip(face::kind_of(&toast.item), look))
                .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_DEFAULT)).child(title.clone()))
                .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(said))
                .child(div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_SUBTLE)).child(clock_of(toast.item.raised_at.as_str())))
                .on_click(cx.listener(move |this, _, window, cx| this.open_operator(id.clone(), window, cx)));
            labelled(row, Role::ListItem, format!("{title}, {said}"))
        });
        let panel = div()
            .id("toast-history")
            .track_focus(&self.history_focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                    cx.stop_propagation();
                    this.toggle_history(window, cx);
                }
            }))
            .absolute()
            .top(look.length(token::H_TITLE_BAR) + look.length(token::SPACE_2))
            .right(look.length(token::SPACE_6))
            .w(gpui::px(WIDTH + 40.))
            .max_h(gpui::px(480.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .p(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_OVERLAY))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            // A press on a toast never reaches what is under it.
            .occlude()
            .font_family(sans(look))
            .child(
                div()
                    .px(look.length(token::SPACE_3))
                    .pb(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_MUTED))
                    .child("Toasts"),
            )
            .children(rows);
        labelled(panel, Role::Dialog, "Toasts")
    }
}

impl Render for Toasts {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        // Where a message box is drawn under the pile it stands above the box, so no control under it is hidden;
        // the box was painted the frame before, so a change draws once more.
        let lift = self.lift_over_box(window, &look, cx);
        if lift != self.lift {
            self.lift = lift;
            window.request_animation_frame();
        }
        // On the Operator's page no toast of the Operator's is drawn: the page holds each item. Fleet's
        // states and the presses Fleet did not take still show.
        let on_page = self.screen.upgrade().is_some_and(|screen| screen.read(cx).page_shown(cx));
        if on_page {
            self.shelf.hush();
        }
        let reading = cx.try_global::<Reading>().is_some_and(|now| now.0);
        let shown: Vec<Toast> = if reading { Vec::new() } else { self.shelf.shown().into_iter().cloned().collect() };
        let last = shown.len().saturating_sub(1);
        // Fleet's states stand first, at the top of the stack: urgent, and held while they last.
        let fleet: Vec<FleetNotice> = cx.try_global::<FleetNotices>().map(|kept| kept.0.clone()).unwrap_or_default().into_iter().chain(signed_out_notice(cx)).collect();
        // Then what the app was told: presses Fleet did not take, each until he closes it.
        let told: Vec<TellOne> = cx.try_global::<Told>().map(|kept| kept.0.clone()).unwrap_or_default();
        // Then what the Operator did with his dispatches and the reviews asked of him, each until he presses it or closes
        // it. The Operator's page holds its own; a review asked of him is not on it.
        let ruled: Vec<Raised> = if reading {
            Vec::new()
        } else {
            cx.try_global::<Rulings>().map(|kept| kept.0.iter().filter(|one| (!on_page || !one.operators()) && !one.hidden()).cloned().collect()).unwrap_or_default()
        };
        // Under the title bar at most: in a short window the oldest is cut at the top, never the bell.
        let mut cards: Vec<AnyElement> = Vec::new();
        cards.extend(fleet.iter().map(|notice| self.fleet_toast(notice, look).into_any_element()));
        cards.extend(told.iter().map(|one| self.told_toast(one, look, cx).into_any_element()));
        cards.extend(ruled.iter().enumerate().map(|(at, one)| self.ruling_toast(one, shown.is_empty() && at + 1 == ruled.len(), look, cx).into_any_element()));
        cards.extend(shown.iter().enumerate().map(|(at, toast)| self.toast(toast, at == last, look, cx).into_any_element()));
        // A pile: only the newest card stands in full, the ones under it peek out above it, and a pointer over the pile fans them out.
        let fanned = self.fanned || cards.len() < 2;
        let mut stack = div()
            .id("toast-pile")
            .absolute()
            .top(look.length(token::H_TITLE_BAR) + look.length(token::SPACE_2))
            .right(look.length(token::SPACE_6))
            .bottom(look.length(token::SPACE_6) + self.lift)
            .flex()
            .flex_col()
            .justify_end()
            .overflow_hidden()
            .gap(look.length(token::SPACE_3))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                if this.fanned != *hovered {
                    this.fanned = *hovered;
                    cx.notify();
                }
            }));
        if fanned {
            stack = stack.children(cards);
        } else {
            let front = cards.pop();
            // The front card sets the box; each card behind is clipped to it, shifted up by its step, so it shows
            // a sliver above the front however tall it is. The farthest is painted first so the nearest sits on top.
            let behind: Vec<AnyElement> = cards.into_iter().rev().take(PEEK_DEPTH).collect();
            let mut pile = div().relative().w(gpui::px(WIDTH));
            for (depth, card) in behind.into_iter().enumerate().rev() {
                let offset = look.length(token::SPACE_2) * (depth + 1) as f32;
                pile = pile.child(div().absolute().top(-offset).bottom_0().right(offset / 2.).w(gpui::px(WIDTH)).overflow_hidden().child(card));
            }
            stack = stack.child(pile.children(front));
        }
        // Deferred, so the pile paints and takes presses over the dispatch sheet's scrim, which is drawn over this
        // screen: a refused send keeps the sheet open, and its failure toast's buttons must still press. Under
        // Settings and the palette (2).
        gpui::deferred(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .child(stack)
                .when(self.history, |layer| layer.child(self.history_panel(look, cx))),
        )
        .with_priority(1)
    }
}

/// The bell in the title bar: the history on a press, a dot while a toast is unseen.
struct Bell {
    toasts: WeakEntity<Toasts>,
    _sub: Subscription,
}

impl Bell {
    fn new(toasts: &Entity<Toasts>, cx: &mut Context<Self>) -> Self {
        Bell { toasts: toasts.downgrade(), _sub: cx.observe(toasts, |_, _, cx| cx.notify()) }
    }
}

impl Render for Bell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let unseen = self.toasts.upgrade().is_some_and(|toasts| toasts.read(cx).unseen);
        let toasts = self.toasts.clone();
        let key = crate::generated::actions::ACTIONS.iter().find(|act| act.id == "toast_history").map(|act| act.shortcut).unwrap_or_default();
        let bell: Stateful<gpui::Div> = icon_button("toast-bell", "bell", look.color(token::FG_MUTED), look)
            .relative()
            .when(unseen, |bell| {
                bell.child(
                    div()
                        .absolute()
                        .right(look.length(token::SPACE_1))
                        .top(look.length(token::SPACE_1))
                        .size(look.length(token::SPACE_2))
                        .rounded_full()
                        .bg(look.color(token::OPERATOR)),
                )
            })
            .tooltip(tip(if unseen { format!("Toasts, one unseen — {key}") } else { format!("Toasts — {key}") }))
            .on_click(move |_, window, cx| {
                if let Some(toasts) = toasts.upgrade() {
                    toasts.update(cx, |toasts, cx| toasts.toggle_history(window, cx));
                }
            });
        labelled(bell, Role::Button, "Toasts")
    }
}

/// The edge: red for urgent, amber for a decision, the accent for a go-ahead.
fn edge_of(urgency: Urgency, look: &Look) -> Hsla {
    look.color(match urgency {
        Urgency::Urgent => token::STATUS_COMPLETED_FAILED,
        Urgency::Decision => token::STATUS_AWAITING_REVIEW,
        Urgency::GoAhead => token::ACCENT,
    })
}

/// What the item asks, where a Job holds it; else the Operator's read.
fn sentence(item: &OperatorTurnItem) -> SharedString {
    face::asks_of(item).or_else(|| item.read.as_ref().map(|read| SharedString::from(read.read.clone()))).unwrap_or_default()
}

/// The muted line: the Operator's read under what a Job asks; else where the item is.
fn muted_line(item: &OperatorTurnItem) -> Option<SharedString> {
    match face::asks_of(item) {
        Some(_) => item.read.as_ref().map(|read| read.read.clone().into()),
        None => item.repository.clone().map(Into::into),
    }
}

/// `14:10:00`, in his clock.
fn clock_of(at: &str) -> String {
    crate::settings::time::stamp_at(at).unwrap_or_default()
}

fn history_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/Armada/native-toasts"))
}

/// One toast a line: how it ended, a tab, the item as Fleet served it.
fn line_of(toast: &Toast) -> Option<String> {
    let ended = match toast.ended {
        None => "open",
        Some(Ended::Settled) => "settled",
        Some(Ended::Closed) => "closed",
        Some(Ended::Later) => "later",
    };
    ipc::encode(&toast.item).ok().map(|item| format!("{ended}\t{item}"))
}

fn read_history(file: &PathBuf) -> Vec<Toast> {
    let Ok(text) = std::fs::read_to_string(file) else { return Vec::new() };
    text.lines()
        .filter_map(|line| {
            let (ended, item) = line.split_once('\t')?;
            let item = ipc::decode::<OperatorTurnItem>("OperatorTurnItem", item.as_bytes()).ok()?;
            let ended = match ended {
                "settled" => Some(Ended::Settled),
                "closed" => Some(Ended::Closed),
                "later" => Some(Ended::Later),
                _ => None,
            };
            Some(Toast { urgency: item.urgency, item, ended })
        })
        .collect()
}

#[cfg(test)]
mod ruling_tests {
    use super::*;

    fn news(way: DispatchWay, title: Option<&str>) -> OperatorDispatched {
        OperatorDispatched { dispatch_id: "d".into(), manifest_id: "armada".into(), way, session_id: None, title: title.map(str::to_string) }
    }

    #[test]
    fn each_way_the_operator_ruled_has_its_own_sentence() {
        assert_eq!(ruling_said(&news(DispatchWay::Started, Some("Fix the build")), None), "Operator started Fix the build");
        assert_eq!(ruling_said(&news(DispatchWay::Handed, Some("Fix the build")), None), "Operator handed it to Fix the build");
        assert_eq!(ruling_said(&news(DispatchWay::Parked, Some("Keep the scrollback")), None), "Operator kept it as an idea");
    }

    #[test]
    fn a_question_names_its_job_else_the_words_he_sent_cut_to_a_line() {
        assert_eq!(ruling_said(&news(DispatchWay::Asked, Some("Fix the build")), None), "Operator asks about Fix the build");
        let long = "Retry the webhook sender on a 503 and keep the dead letters for a day in sample-api";
        let said = ruling_said(&news(DispatchWay::Asked, None), Some(about(long)));
        assert_eq!(said, "Operator asks about Retry the webhook sender on a 503 and keep the d...");
        assert_eq!(about("one line\nsecond"), "one line");
    }

    /// 14 Nov: a dispatch from the sheet came back as `Dispatched`, and the toasts' own subscriber updated the
    /// toasts again from inside their update, which gpui aborts on.
    #[gpui::test]
    fn a_dispatch_heard_while_the_toasts_are_up_rings_the_bell_without_a_panic(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        let (page, cx) = cx.add_window_view(|window, cx| OperatorPageView::mock(window, cx));
        let toasts = cx.update(|window, cx| Toasts::mount(&page, WeakEntity::new_invalid(), window, cx));
        let fleet = cx.update(|_, cx| cx.new(crate::fleet::FleetConnection::idle));
        let heard = cx.update(|_, cx| toasts.update(cx, |_, cx| Toasts::hear_dispatches(&fleet, cx)));
        fleet.update(cx, |_, cx| cx.emit(crate::fleet::Dispatched(vec![news(DispatchWay::Started, Some("Fix the build"))])));
        cx.run_until_parked();
        toasts.read_with(cx, |toasts, _| assert!(toasts.unseen, "the bell says a ruling is new"));
        let said = cx.update(|_, cx| cx.global::<Rulings>().0.iter().map(|one| one.sentence.to_string()).collect::<Vec<_>>());
        assert_eq!(said, ["Operator started Fix the build"]);
        drop(heard);
    }
}

#[cfg(test)]
mod sign_in_tests {
    use super::*;

    fn out(since: &str) -> AgentSignIn {
        AgentSignIn { signed_out_since: Some(ipc::Instant::carried(since)), ..AgentSignIn::default() }
    }

    #[test]
    fn a_signed_out_spell_is_news_once_and_never_on_connecting() {
        let signed_in = AgentSignIn::default();
        // Found signed out on connecting: the toast stands, no banner.
        assert!(!signed_out_news(None, &out("09:00")));
        // Signed out after being signed in: news.
        assert!(signed_out_news(Some(&signed_in), &out("09:00")));
        // The same spell again, as signing in starts: not news.
        let signing = AgentSignIn { signing_in: true, ..out("09:00") };
        assert!(!signed_out_news(Some(&out("09:00")), &signing));
        // Signed back in: not news; a new spell later is.
        assert!(!signed_out_news(Some(&out("09:00")), &signed_in));
        assert!(signed_out_news(Some(&out("09:00")), &out("11:30")));
    }
}

#[cfg(test)]
mod reading_tests {
    use super::*;

    struct Root(Entity<Toasts>);

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some()))
    }

    #[gpui::test]
    fn while_he_reads_the_operators_toasts_wait_and_fleets_states_still_show(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            at_rest(cx);
        });
        let (_, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| OperatorPageView::mock(window, cx));
            Root(Toasts::mount(&page, WeakEntity::new_invalid(), window, cx))
        });
        cx.update(|_, cx| {
            dispatched(OperatorDispatched { dispatch_id: "d".into(), manifest_id: "armada".into(), way: DispatchWay::Started, session_id: None, title: Some("Fix the build".into()) }, cx);
            fleet(vec![FleetNotice { id: "slow", headline: "Fleet is slow".into(), next: "Wait".into(), code: "slow", fault: false, act: None }], cx);
        });
        let redraw = |cx: &mut gpui::VisualTestContext| {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        };
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Operator started Fix the build"), "a ruling shows while he is not reading");
        cx.update(|_, cx| reading(true, cx));
        redraw(cx);
        redraw(cx);
        assert!(!drawn(cx, Role::Button, "Operator started Fix the build"), "it waits while he reads");
        assert!(drawn(cx, Role::Alert, "Toast: Fleet is slow"), "a state of Fleet still shows");
        cx.update(|_, cx| reading(false, cx));
        redraw(cx);
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Operator started Fix the build"), "and it shows when he is done");
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::refusal::{tests::TOO_LARGE, Failed};

    struct Root(Entity<Toasts>);

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> bool {
        cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().is_some_and(|book| book.find(window.window_handle().window_id(), role, name).is_some()))
    }

    fn redraw(cx: &mut gpui::VisualTestContext) {
        for _ in 0..2 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    /// What the dispatch sheet does with Fleet's answer to its Start (`DispatchScreen::dispatched`): a refusal
    /// raises the failure toast under "The Job was not dispatched", and a call that never reached Fleet does not.
    #[gpui::test]
    fn a_refused_dispatch_raises_the_failure_toast_with_its_route_status_and_sentence(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            at_rest(cx);
        });
        let (_, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| OperatorPageView::mock(window, cx));
            Root(Toasts::mount(&page, WeakEntity::new_invalid(), window, cx))
        });
        let refused = crate::refusal::answered("POST", "/sessions/dispatch", Some(14_900_000), 413, TOO_LARGE.to_vec()).expect_err("Fleet refused it");
        cx.update(|_, cx| {
            Failed::Unanswered("Fleet is not connected".into()).tell_refusal("The Job was not dispatched", cx);
            Failed::Refused(refused).tell_refusal("The Job was not dispatched", cx);
        });
        redraw(cx);
        let told: Vec<String> = cx.update(|_, cx| cx.try_global::<Told>().map(|kept| kept.0.iter().map(|one| one.sentence.to_string()).collect()).unwrap_or_default());
        assert_eq!(told, ["The Job was not dispatched: Fleet refused it as too large"], "one toast, for the refusal alone");
        assert!(drawn(cx, Role::Alert, "Toast: The Job was not dispatched: Fleet refused it as too large"), "the failure toast is drawn");
        assert!(drawn(cx, Role::Label, "POST /sessions/dispatch carried 14.2 MB; Fleet takes at most 10 MB."), "Fleet's own sentence");
        assert!(drawn(cx, Role::Label, "Route: POST /sessions/dispatch"), "the route row");
        assert!(drawn(cx, Role::Label, "Status: 413 request.too_large"), "the status row, with Fleet's code");
        assert!(drawn(cx, Role::Label, "Fleet log: Not shown; sent with the Job"), "the log row");
        for button in ["Copy debug info", "Dismiss", "Dispatch job"] {
            assert!(drawn(cx, Role::Button, button), "{button}");
        }
    }

    fn mounted(cx: &mut gpui::TestAppContext) -> &mut gpui::VisualTestContext {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            at_rest(cx);
        });
        let (_, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| OperatorPageView::mock(window, cx));
            Root(Toasts::mount(&page, WeakEntity::new_invalid(), window, cx))
        });
        cx
    }

    fn told_sentences(cx: &mut gpui::VisualTestContext) -> Vec<String> {
        cx.update(|_, cx| cx.try_global::<Told>().map(|kept| kept.0.iter().map(|one| one.sentence.to_string()).collect()).unwrap_or_default())
    }

    /// Fleet's routes hang while its stream stays up: once the one held notice stands, a request that
    /// timed out raises no toast of its own.
    #[gpui::test]
    fn a_timed_out_request_raises_no_toast_while_fleet_is_slow(cx: &mut gpui::TestAppContext) {
        let cx = mounted(cx);
        cx.update(|_, cx| tell("Your answer did not reach Fleet: no answer in 30 seconds", cx));
        assert_eq!(told_sentences(cx).len(), 1, "before the notice stands, the failure is told");
        cx.update(|_, cx| {
            at_rest(cx);
            fleet(vec![crate::titlebar::slow_notice(true).expect("slow")], cx);
            tell("Your answer did not reach Fleet: no answer in 30 seconds", cx);
            tell_retry("The message did not reach Fleet: no answer in 30 seconds", std::rc::Rc::new(|_: &mut App| {}), cx);
            tell("Fleet refused it", cx);
        });
        assert_eq!(told_sentences(cx), ["Fleet refused it"], "only the timeouts are held back");
        redraw(cx);
        assert!(drawn(cx, Role::Alert, "Toast: Fleet is slow"), "the one notice is drawn");
    }

    /// The pile: a taller card under a shorter newest one peeks out by its step only, never by its whole height.
    #[gpui::test]
    fn a_taller_card_under_the_newest_peeks_out_by_its_step_only(cx: &mut gpui::TestAppContext) {
        let cx = mounted(cx);
        let tall = "Your answer did not reach Fleet because the connection dropped while it was being sent, and the sentence runs over several lines of the card so it is much taller than the newest";
        cx.update(|_, cx| {
            tell(tall, cx);
            tell("Short", cx);
        });
        redraw(cx);
        let (front, behind) = cx.update(|window, cx| {
            let book = cx.try_global::<crate::labels::LabelBook>().expect("labels");
            let front = book.find(window.window_handle().window_id(), Role::Alert, "Toast: Short").expect("front").bounds;
            let behind = book.find(window.window_handle().window_id(), Role::Alert, &format!("Toast: {tall}")).expect("behind").bounds;
            (front, behind)
        });
        assert!(behind.size.height > front.size.height, "the card under is the taller");
        let peek = front.origin.y - behind.origin.y;
        assert!(peek > gpui::px(0.) && peek <= gpui::px(16.), "it peeks above the front by a step, not by {peek:?}");
    }
}


#[cfg(test)]
mod review_tests {
    use super::*;
    use crate::operator::screen::OperatorScreen;
    use ipc::{CallArrival, CallArrivalKind, SessionId, StandingCalls};

    const KEY: &str = "review:harbor/ledger-api#2231";
    const TITLE: &str = "Say which stage of an invoice failed";
    const SAID: &str = "Review requested on harbor/ledger-api#2231, opened by mira";

    fn asked() -> ipc::ReviewAsked {
        ipc::ReviewAsked {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            repository: "harbor/ledger-api".into(),
            number: 2231,
            title: TITLE.into(),
            author: "mira".into(),
            address: "https://example.com/harbor/ledger-api/pull/2231".into(),
        }
    }

    /// Fleet's calls with the one review asked of him, opened or not.
    fn calls(opened: bool) -> StandingCalls {
        let review = r#"{"manifest_id":"ledger-api","repository":"harbor/ledger-api","number":2231,"title":"Say which stage of an invoice failed","author":"mira","address":"https://example.com/harbor/ledger-api/pull/2231"}"#;
        let call = format!(
            r#"{{"key":"{KEY}","owner":"{KEY}","kind":"review_requested","title":"{TITLE}","text":"{SAID}","since":"2026-11-16T09:00:00.000Z","tone":"waiting","opened":{opened},"review":{review}}}"#
        );
        ipc::decode("StandingCalls", format!(r#"{{"calls":[{call}]}}"#).as_bytes()).expect("the calls decode")
    }

    fn arrival() -> CallArrival {
        CallArrival { session_id: SessionId::carried(KEY), title: TITLE.into(), text: SAID.into(), tone: CallTone::Waiting, kind: CallArrivalKind::ReviewRequested, key: Some(KEY.into()), item: None }
    }

    struct Root(gpui::AnyView);

    impl Render for Root {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            crate::labels::label_root(div().size_full().child(self.0.clone()))
        }
    }

    fn ready(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
            crate::titlebar::bind(cx);
            at_rest(cx);
        });
    }

    fn redraw(cx: &mut gpui::VisualTestContext) {
        for _ in 0..2 {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        }
    }

    fn drawn(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Option<gpui::Bounds<gpui::Pixels>> {
        cx.update(|window, cx| cx.try_global::<crate::labels::LabelBook>().and_then(|book| book.find(window.window_handle().window_id(), role, name)).map(|one| one.shown))
    }

    fn raised(cx: &mut gpui::VisualTestContext) -> Vec<String> {
        cx.update(|_, cx| cx.try_global::<Rulings>().map(|kept| kept.0.iter().map(|one| one.sentence.to_string()).collect()).unwrap_or_default())
    }

    #[gpui::test]
    fn a_review_asked_of_him_raises_one_toast_that_goes_once_he_opens_it(cx: &mut gpui::TestAppContext) {
        ready(cx);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| OperatorPageView::mock(window, cx));
            Root(Toasts::mount(&page, WeakEntity::new_invalid(), window, cx).into())
        });
        let toasts: Entity<Toasts> = root.read_with(cx, |root, _| root.0.clone().downcast().expect("the toasts"));
        let fleet = cx.update(|_, cx| cx.new(crate::fleet::FleetConnection::idle));
        let heard = cx.update(|_, cx| toasts.update(cx, |_, cx| Toasts::hear_reviews(&fleet, cx)));
        fleet.update(cx, |fleet, cx| {
            fleet.hold_calls(calls(false));
            cx.notify();
            cx.emit(crate::fleet::Arrived(vec![arrival()]));
        });
        redraw(cx);
        assert_eq!(raised(cx), [TITLE], "the toast names the pull request");
        assert!(drawn(cx, Role::Alert, &format!("Toast: {TITLE}")).is_some(), "it is drawn");
        fleet.update(cx, |_, cx| cx.emit(crate::fleet::Arrived(vec![arrival()])));
        redraw(cx);
        assert_eq!(raised(cx), [TITLE], "a request told again while its toast stands raises no second one");
        fleet.update(cx, |fleet, cx| {
            fleet.hold_calls(calls(true));
            cx.notify();
        });
        redraw(cx);
        assert!(raised(cx).is_empty(), "opened anywhere, its toast goes");
        drop(heard);
    }

    #[gpui::test]
    fn pressing_a_review_toast_opens_that_pull_request_in_reviews(cx: &mut gpui::TestAppContext) {
        ready(cx);
        let (root, cx) = cx.add_window_view(|window, cx| {
            let records = crate::mock::jobs("jobs-many").expect("the fixture decodes");
            let list = cx.new(|cx| crate::jobs::JobsList::mock(&records, cx));
            let jobs = cx.new(|cx| crate::jobs::JobsScreen::new(list, None, window, cx));
            Root(cx.new(|cx| OperatorScreen::mock_reviews(jobs, window, cx)).into())
        });
        let screen: Entity<OperatorScreen> = root.read_with(cx, |root, _| root.0.clone().downcast().expect("the screen"));
        cx.update(|_, cx| review_asked(KEY, asked(), SAID, cx));
        redraw(cx);
        let press = drawn(cx, Role::Button, TITLE).expect("the toast's press is drawn");
        cx.simulate_click(press.center(), gpui::Modifiers::none());
        redraw(cx);
        assert_eq!(screen.read_with(cx, |screen, _| screen.front), Front::Reviews, "Reviews comes to the front");
        let reviews = cx.update(|_, cx| crate::reviews::walked(cx)).expect("the reviews screen");
        assert_eq!(reviews.read_with(cx, |reviews, _| reviews.showing()), Some(2231), "on the pull request asked of him");
        assert!(raised(cx).is_empty(), "the pressed toast goes");
    }

    fn tools() -> Vec<ReviewTool> {
        let tool = |id: &str, name: &str, said: &str, first: bool| ReviewTool { id: id.into(), name: name.into(), said: said.into(), first };
        vec![tool("bugs", "Possible bugs", "possible bugs", true), tool("risks", "Risks", "risks", true), tool("api", "API breaking changes", "API breaking changes", false)]
    }

    /// The toasts hearing an idle Fleet, its review routes stood in for, with the review asked of him raised.
    fn asked_toast(cx: &mut gpui::TestAppContext) -> (Entity<crate::fleet::FleetConnection>, Vec<Subscription>, &mut gpui::VisualTestContext) {
        ready(cx);
        cx.update(|cx| cx.set_global(FakeReviews { tools: tools(), posted: Vec::new() }));
        let (root, cx) = cx.add_window_view(|window, cx| {
            let page = cx.new(|cx| OperatorPageView::mock(window, cx));
            Root(Toasts::mount(&page, WeakEntity::new_invalid(), window, cx).into())
        });
        let toasts: Entity<Toasts> = root.read_with(cx, |root, _| root.0.clone().downcast().expect("the toasts"));
        let fleet = cx.update(|_, cx| cx.new(crate::fleet::FleetConnection::idle));
        let heard = cx.update(|_, cx| toasts.update(cx, |_, cx| Toasts::hear_reviews(&fleet, cx)));
        cx.update(|_, cx| review_asked(KEY, asked(), SAID, cx));
        redraw(cx);
        (fleet, heard, cx)
    }

    fn press_on(cx: &mut gpui::VisualTestContext, role: Role, name: &str) {
        let at = drawn(cx, role, name).unwrap_or_else(|| panic!("{name} is drawn"));
        cx.simulate_click(at.center(), gpui::Modifiers::none());
        redraw(cx);
    }

    fn stage(cx: &mut gpui::VisualTestContext) -> Option<Stage> {
        cx.update(|_, cx| cx.global::<Rulings>().0.iter().find_map(|one| match &one.opens {
            Opens::Review { stage, .. } => Some(stage.clone()),
            _ => None,
        }))
    }

    fn details(cx: &mut gpui::VisualTestContext) -> Vec<(String, String)> {
        cx.update(|_, cx| cx.global::<Rulings>().0.iter().map(|one| (one.sentence.to_string(), one.detail.clone().unwrap_or_default().to_string())).collect())
    }

    /// A tool's run as Fleet tells it: `done` with this many findings, or `failed`.
    fn run(tool: &str, found: Option<usize>) -> ipc::ReviewRun {
        let finding = r#"{"title":"Retry can loop","body":"b","severity":"medium","at":{"path":"src/retry.rs","side":"right","start":3,"end":4}}"#;
        let ended = match found {
            Some(count) => format!(r#""state":"done","result":{{"shape":"findings","findings":[{}]}}"#, vec![finding; count].join(",")),
            None => r#""state":"failed","failed":"the bay stopped""#.to_string(),
        };
        let run = format!(r#"{{"manifest_id":"ledger-api","number":2231,"kind":"tool","name":"{tool}","head":"f00d","started_at":"2026-11-19T09:00:00.000Z",{ended}}}"#);
        ipc::decode("ReviewRun", run.as_bytes()).expect("the run decodes")
    }

    fn start_with_first_tools(cx: &mut gpui::VisualTestContext) {
        press_on(cx, Role::Button, "Review…");
        press_on(cx, Role::Button, "Start review");
    }

    #[gpui::test]
    fn review_opens_the_picker_with_the_tools_he_used_last_on_that_repository_ticked(cx: &mut gpui::TestAppContext) {
        let (_fleet, _heard, cx) = asked_toast(cx);
        cx.update(|_, cx| crate::reviews::keep_ask("harbor/ledger-api".into(), ReviewAsk { tools: vec!["risks".into(), "api".into()], ..ReviewAsk::first(&[]) }, cx));
        press_on(cx, Role::Button, "Review…");
        for tool in ["Possible bugs", "Risks", "API breaking changes"] {
            assert!(drawn(cx, Role::CheckBox, tool).is_some(), "{tool} has its checkbox in the toast");
        }
        assert!(drawn(cx, Role::Button, "Cancel").is_some() && drawn(cx, Role::Button, "Start review").is_some(), "Cancel and Start review under them");
        let Some(Stage::Picking { ask, .. }) = stage(cx) else { panic!("the picker is open") };
        assert_eq!(ask.tools, ["risks", "api"], "ticked as he last asked on ledger-api");
    }

    #[gpui::test]
    fn start_review_asks_one_run_a_ticked_tool_and_says_it_runs_in_the_background(cx: &mut gpui::TestAppContext) {
        let (fleet, _heard, cx) = asked_toast(cx);
        start_with_first_tools(cx);
        let posted = cx.update(|_, cx| cx.global::<FakeReviews>().posted.clone());
        let asked: Vec<(&str, &str, bool, ipc::ReviewRunKind)> = posted.iter().map(|(manifest, run)| (manifest.as_str(), run.name.as_str(), run.again, run.kind)).collect();
        assert_eq!(asked, [("ledger-api", "bugs", true, ipc::ReviewRunKind::Tool), ("ledger-api", "risks", true, ipc::ReviewRunKind::Tool)], "one run a ticked tool");
        assert_eq!(details(cx), [("Review running in the background".to_string(), "Possible bugs and risks on #2231. You'll get a notification when it's ready.".to_string())]);
        assert!(drawn(cx, Role::Image, "Working").is_some(), "the running state's glyph");
        assert!(drawn(cx, Role::Button, "Review running in the background").is_some(), "the running toast is drawn");
        fleet.update(cx, |fleet, cx| {
            fleet.hold_calls(calls(true));
            cx.notify();
        });
        redraw(cx);
        assert_eq!(details(cx).len(), 1, "opening the pull request elsewhere leaves the running toast");
    }

    #[gpui::test]
    fn every_run_ended_raises_the_ready_toast_with_its_findings_and_drops_the_running_one(cx: &mut gpui::TestAppContext) {
        let (fleet, _heard, cx) = asked_toast(cx);
        start_with_first_tools(cx);
        fleet.update(cx, |_, cx| cx.emit(crate::fleet::ReviewRunsMoved { runs: vec![run("bugs", Some(2))], lost: false }));
        redraw(cx);
        assert_eq!(raised(cx), ["Review running in the background"], "one run still going");
        fleet.update(cx, |_, cx| cx.emit(crate::fleet::ReviewRunsMoved { runs: vec![run("risks", Some(1))], lost: false }));
        redraw(cx);
        assert_eq!(details(cx), [(format!("Review ready: {TITLE}"), "3 findings from possible bugs and risks.".to_string())], "the ready toast alone");
        assert!(drawn(cx, Role::Button, &format!("Review ready: {TITLE}")).is_some(), "it is drawn");
    }

    #[gpui::test]
    fn a_failed_run_is_named_in_the_ready_line(cx: &mut gpui::TestAppContext) {
        let (fleet, _heard, cx) = asked_toast(cx);
        start_with_first_tools(cx);
        fleet.update(cx, |_, cx| cx.emit(crate::fleet::ReviewRunsMoved { runs: vec![run("bugs", Some(1)), run("risks", None)], lost: false }));
        redraw(cx);
        let ready = details(cx).into_iter().find(|(sentence, _)| sentence.starts_with("Review ready")).map(|(_, line)| line);
        assert_eq!(ready.as_deref(), Some("1 finding from possible bugs. Risks failed."));
    }
}
