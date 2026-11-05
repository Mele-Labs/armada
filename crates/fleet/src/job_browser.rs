//! The Job browser's calls, each held until Bridge answers it. **Never written down**: a held call
//! is an agent's open HTTP request, which a restart cuts anyway.
//! `docs/practices/protocol/the-job-browser.md`.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{
    AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserAskAnswered, BrowserAskNoted, BrowserError, BrowserRequest, BrowserRequestAnswered, EndBrowserAsk,
    RaiseBrowserAsk, SessionId, WalkTarget, WireError,
};
use tokio::sync::oneshot;

use crate::daemon::Fleet;
use crate::mint::{Mint, UlidMint};

/// How long Fleet holds an act for Bridge. With `wait_for`'s own wait on top, at most 50 seconds:
/// a hosted Job's harness ends a call over HTTP at 60 (spike 15).
const ACT_WITHIN: Duration = Duration::from_secs(20);
/// `wait_for`'s wait where it named none, and the most it may name.
const WAIT_MS: u64 = 5_000;
const MOST_WAIT_MS: u64 = 30_000;
/// The most of a snapshot and of an evaluated value the agent is answered: inside the door's 64 KiB.
const MOST_SNAPSHOT: usize = 48 * 1024;
const MOST_RESULT: usize = 32 * 1024;
/// How many ended ids are remembered, so a late answer is refused with why rather than as unknown.
const ENDED_KEPT: usize = 64;

const REFUSED: &str = "fleet.browser_refused";
const UNPLACED: &str = "fleet.browser_unplaced";
const NO_BRIDGE: &str = "fleet.browser_no_bridge";
const TARGET_NOT_FOUND: &str = "fleet.browser_target_not_found";
const TIMED_OUT: &str = "fleet.browser_timed_out";
const FAILED: &str = "fleet.browser_failed";
const UNANSWERED: &str = "fleet.browser_unanswered";
const NO_SUCH_REQUEST: &str = "fleet.no_such_browser_request";
const WITHDRAWN: &str = "fleet.browser_request_withdrawn";
const ANSWERED: &str = "fleet.browser_request_answered";
const ANSWER_REFUSED: &str = "fleet.browser_answer_refused";
const ASK_REFUSED: &str = "fleet.browser_ask_refused";

/// The front of a page ask's item id, the rest being Bridge's tab id.
pub(crate) const ASK_ITEM: &str = "browser_ask:";

pub(crate) const UNANSWERED_SAID: &str = "Bridge did not answer. Is the Job browser open?";

type Done = Result<BrowserAnswer, BrowserError>;

/// A page's ask for the owner, as Bridge raised it.
#[derive(Clone)]
struct PageAsk {
    session: String,
    id: String,
    text: String,
    options: Vec<ipc::WaitingOption>,
    since: String,
}

struct Held {
    session: String,
    act: BrowserAct,
    answer: oneshot::Sender<Done>,
}

#[derive(Clone, Copy)]
enum Ended {
    Answered,
    Withdrawn,
}

/// The calls held for Bridge, by request id, and the last ids that ended.
pub(crate) struct Browsers {
    held: Mutex<BTreeMap<String, Held>>,
    ended: Mutex<VecDeque<(String, Ended)>>,
    /// Pages' asks in raise order. Memory only: a restart loses them, and Bridge still has its own.
    asks: Mutex<Vec<PageAsk>>,
    mint: UlidMint,
    stopping: AtomicBool,
    within_ms: AtomicU64,
}

impl Default for Browsers {
    fn default() -> Browsers {
        Browsers {
            held: Mutex::default(),
            ended: Mutex::default(),
            asks: Mutex::default(),
            mint: UlidMint::new(),
            stopping: AtomicBool::new(false),
            within_ms: AtomicU64::new(ACT_WITHIN.as_millis() as u64),
        }
    }
}

impl Browsers {
    /// Fleet is stopping: every held call is let go, and none is held again.
    pub(crate) fn all_withdrawn(&self) {
        if let Ok(mut held) = self.held.lock() {
            self.stopping.store(true, Ordering::SeqCst);
            held.clear();
        }
    }

    #[cfg(test)]
    pub(crate) fn held_within(&self, within: Duration) {
        self.within_ms.store(within.as_millis() as u64, Ordering::SeqCst);
    }

    fn within(&self) -> Duration {
        Duration::from_millis(self.within_ms.load(Ordering::SeqCst))
    }

    fn minted(&self) -> String {
        format!("browser-{}", self.mint.ulid().as_str())
    }

    /// Hold `act` under `id`. `None` once Fleet is stopping.
    fn hold(&self, id: &str, session: &str, act: BrowserAct) -> Option<oneshot::Receiver<Done>> {
        let mut held = self.held.lock().ok().filter(|_| !self.stopping.load(Ordering::SeqCst))?;
        let (answer, answered) = oneshot::channel();
        held.insert(id.to_string(), Held { session: session.to_string(), act, answer });
        Some(answered)
    }

    /// Hold `ask`, replacing the one of its tab: the place and the first `since` are kept.
    fn ask_held(&self, mut ask: PageAsk) {
        let Ok(mut asks) = self.asks.lock() else { return };
        match asks.iter_mut().find(|one| one.session == ask.session && one.id == ask.id) {
            Some(held) => {
                ask.since = std::mem::take(&mut held.since);
                *held = ask;
            }
            None => asks.push(ask),
        }
    }

    fn ask_taken(&self, session: &str, id: &str) -> Option<PageAsk> {
        let mut asks = self.asks.lock().ok()?;
        let at = asks.iter().position(|one| one.session == session && one.id == id)?;
        Some(asks.remove(at))
    }

    /// The Job's asks, or none where nobody listens, which also forgets every Job's.
    fn asks_of(&self, session: &str, listening: bool) -> Vec<PageAsk> {
        let Ok(mut asks) = self.asks.lock() else { return Vec::new() };
        if !listening {
            asks.clear();
        }
        asks.iter().filter(|one| one.session == session).cloned().collect()
    }

    /// Take `id` off the table because its wait ran out.
    fn withdraw(&self, id: &str) {
        let taken = self.held.lock().ok().and_then(|mut held| held.remove(id));
        if taken.is_some() {
            self.ended(id, Ended::Withdrawn);
        }
    }

    fn ended(&self, id: &str, how: Ended) {
        if let Ok(mut ended) = self.ended.lock() {
            ended.push_back((id.to_string(), how));
            while ended.len() > ENDED_KEPT {
                ended.pop_front();
            }
        }
    }

    fn how_ended(&self, id: &str) -> Option<Ended> {
        let ended = self.ended.lock().ok()?;
        ended.iter().rev().find(|(one, _)| one == id).map(|(_, how)| *how)
    }
}

/// Why an answer was not taken.
enum NotTaken {
    Unknown,
    Gone(Ended),
    Unfit(String),
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `act` held for Bridge, published as `browser.request`, and answered with what Bridge did.
    pub(crate) async fn browser_driven(
        &self,
        caller: Option<api::Caller>,
        named: Option<SessionId>,
        act: BrowserAct,
    ) -> Result<BrowserAnswer, Refusal> {
        let refused = |code: &'static str, why: String| Refusal::Unacceptable(WireError::raised(code, why, self.run_id()));
        let stuck = |code: &'static str, why: String| Refusal::IllegalMove(WireError::raised(code, why, self.run_id()));
        let session = caller
            .and_then(|caller| self.session_holding(&caller))
            .or_else(|| named.map(|named| named.as_str().trim().to_string()))
            .filter(|id| !id.is_empty())
            .ok_or_else(|| refused(UNPLACED, String::from("Fleet hosts no job on this connection, so name yours with session_id")))?;
        let id = self.job_browser().minted();
        let act = self.act_held(&session, &id, act).map_err(|why| refused(REFUSED, why))?;
        if self.events().watching() == 0 {
            return Err(stuck(NO_BRIDGE, String::from("Bridge is not open, so nothing can drive the Job browser. Ask the owner to open Bridge.")));
        }
        let waited = match &act {
            BrowserAct::WaitFor { timeout_ms, .. } => Duration::from_millis(*timeout_ms),
            _ => Duration::ZERO,
        };
        let Some(answered) = self.job_browser().hold(&id, &session, act.clone()) else {
            return Err(stuck(UNANSWERED, String::from("Fleet is stopping, so the Job browser was let go.")));
        };
        self.publish(ipc::Event::BrowserRequest(BrowserRequest {
            session_id: SessionId::carried(&session),
            request_id: id.clone(),
            act: act.clone(),
        }));
        let done = match tokio::time::timeout(self.job_browser().within() + waited, answered).await {
            Ok(Ok(done)) => done,
            Ok(Err(_)) => return Err(stuck(UNANSWERED, String::from("Fleet is stopping, so the Job browser was let go."))),
            Err(_) => {
                self.job_browser().withdraw(&id);
                return Err(stuck(UNANSWERED, String::from(UNANSWERED_SAID)));
            }
        };
        match done {
            Ok(answer) => Ok(cut(answer)),
            Err(BrowserError::NotFound) => Err(refused(TARGET_NOT_FOUND, not_found(&act))),
            Err(BrowserError::TimedOut) => Err(refused(TIMED_OUT, timed_out(&act))),
            Err(BrowserError::Failed { why }) => Err(refused(FAILED, format!("The Job browser could not do that: {}", why.trim()))),
        }
    }

    /// `act` as Fleet holds it, or why it cannot work: a screenshot's file named, a wait's time held.
    fn act_held(&self, session: &str, id: &str, act: BrowserAct) -> Result<BrowserAct, String> {
        Ok(match act {
            BrowserAct::Open { url } => {
                let url = url.trim().to_string();
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    return Err(format!("the Job browser opens http and https addresses only, not {url:?}"));
                }
                BrowserAct::Open { url }
            }
            BrowserAct::Click { target } => BrowserAct::Click { target: aimed(target)? },
            BrowserAct::Type { target, text, submit } => {
                if text.is_empty() && !submit {
                    return Err(String::from("nothing to type: text is empty and submit is false"));
                }
                BrowserAct::Type { target: aimed(target)?, text, submit }
            }
            BrowserAct::Snapshot => BrowserAct::Snapshot,
            BrowserAct::Screenshot { .. } => {
                let folder = self.scratch_of(session).join("browser");
                std::fs::create_dir_all(&folder).map_err(|why| format!("Fleet could not make {}: {why}", folder.display()))?;
                BrowserAct::Screenshot { path: folder.join(format!("{id}.png")).to_string_lossy().into_owned() }
            }
            BrowserAct::WaitFor { text, target, timeout_ms } => {
                let text = text.filter(|text| !text.trim().is_empty());
                let target = target.map(aimed).transpose()?;
                if text.is_some() == target.is_some() {
                    return Err(String::from("wait for text or for a target, one of the two"));
                }
                let timeout_ms = match timeout_ms {
                    0 => WAIT_MS,
                    asked => asked.min(MOST_WAIT_MS),
                };
                BrowserAct::WaitFor { text, target, timeout_ms }
            }
            BrowserAct::Evaluate { expression } => {
                if expression.trim().is_empty() {
                    return Err(String::from("the expression is blank"));
                }
                BrowserAct::Evaluate { expression }
            }
        })
    }

    /// Bridge's answer to one held call.
    pub(crate) async fn browser_request_answered(&self, said: AnswerBrowserRequest) -> Result<BrowserRequestAnswered, Refusal> {
        let raised = |code: &'static str, why: String| WireError::raised(code, why, self.run_id());
        let id = said.request_id.trim().to_string();
        let done: Done = match (said.answer, said.error) {
            (Some(answer), None) => Ok(answer),
            (None, Some(error)) => Err(error),
            _ => {
                return Err(Refusal::Unacceptable(raised(ANSWER_REFUSED, String::from("an answer carries one of answer and error"))));
            }
        };
        let browsers = self.job_browser();
        let taken = {
            let mut held = browsers.held.lock().map_err(|_| Refusal::Fault(raised(ANSWER_REFUSED, String::from("the held calls would not read"))))?;
            match held.remove(&id) {
                None => Err(browsers.how_ended(&id).map(NotTaken::Gone).unwrap_or(NotTaken::Unknown)),
                // Put back, so a corrected answer can still land.
                Some(one) => match unfit(&one.act, &done) {
                    Some(why) => {
                        held.insert(id.clone(), one);
                        Err(NotTaken::Unfit(why))
                    }
                    None => Ok(one),
                },
            }
        };
        match taken {
            Ok(one) => {
                browsers.ended(&id, Ended::Answered);
                // The receiver is gone where its wait ran out in between; the call answered itself.
                let _ = one.answer.send(done.map(|answer| with_path(answer, &one.act)));
                Ok(BrowserRequestAnswered { request_id: id, session_id: SessionId::carried(one.session) })
            }
            Err(NotTaken::Unknown) => Err(Refusal::NotFound(raised(NO_SUCH_REQUEST, format!("Fleet is holding no browser request {id}")))),
            Err(NotTaken::Gone(Ended::Withdrawn)) => Err(Refusal::IllegalMove(raised(
                WITHDRAWN,
                format!("{id} was withdrawn: Bridge took longer than Fleet holds a call, and the agent was told Bridge did not answer"),
            ))),
            Err(NotTaken::Gone(Ended::Answered)) => Err(Refusal::IllegalMove(raised(ANSWERED, format!("{id} was answered already")))),
            Err(NotTaken::Unfit(why)) => Err(Refusal::Unacceptable(raised(ANSWER_REFUSED, why))),
        }
    }
}

impl<H, V, W> Fleet<H, V, W>
where
    H: AgentHarness + Send + Sync + 'static,
    H::Error: std::error::Error + Send + Sync + 'static,
    V: Vcs + Delivery + Send + Sync + 'static,
    V::Error: std::error::Error + Send + Sync + 'static,
    V::CommitError: std::error::Error + Send + Sync + 'static,
    W: WorkProduct + Send + Sync + 'static,
    W::Error: std::error::Error + Send + Sync + 'static,
{
    /// `raise_browser_ask`: a page's ask held in the Job's Needs you. The agent is told nothing.
    pub(crate) async fn browser_ask_raised(&self, ask: RaiseBrowserAsk) -> Result<BrowserAskNoted, Refusal> {
        let (id, text) = (ask.ask_id.trim().to_string(), ask.text.trim().to_string());
        if id.is_empty() || text.is_empty() {
            let why = String::from("a page's ask carries the tab's id and the words it asks");
            return Err(Refusal::Unacceptable(WireError::raised(ASK_REFUSED, why, self.run_id())));
        }
        let session = ask.session_id.as_str().to_string();
        if self.store().lock().await.session(&session).ok().flatten().is_none() {
            let why = format!("Fleet knows no job {session}");
            return Err(Refusal::NotFound(WireError::raised(ASK_REFUSED, why, self.run_id())));
        }
        let since = self.clock().now().as_str().to_string();
        self.job_browser().ask_held(PageAsk { session: session.clone(), id: id.clone(), text, options: ask.options, since });
        self.browser_asks_published(&session).await?;
        Ok(BrowserAskNoted { session_id: ask.session_id, ask_id: id })
    }

    /// `end_browser_ask`: the tab settled it or closed. Nothing is published to the tab.
    pub(crate) async fn browser_ask_ended(&self, ask: EndBrowserAsk) -> Result<BrowserAskNoted, Refusal> {
        let (session, id) = (ask.session_id.as_str().to_string(), ask.ask_id.trim().to_string());
        if self.job_browser().ask_taken(&session, &id).is_some() {
            self.browser_asks_published(&session).await?;
        }
        Ok(BrowserAskNoted { session_id: ask.session_id, ask_id: id })
    }

    /// The owner answered `choice`, or `None` dismissed, the ask under `item`. The tab is told, the
    /// item goes, and the agent is told nothing.
    pub(crate) async fn browser_ask_settled(&self, session: &str, item: &str, choice: Option<u32>) -> Result<ipc::SessionRecord, Refusal> {
        let id = item.strip_prefix(ASK_ITEM).unwrap_or(item);
        if self.job_browser().ask_taken(session, id).is_none() {
            return Err(self.settled_already());
        }
        self.publish(ipc::Event::BrowserAskAnswered(BrowserAskAnswered {
            session_id: SessionId::carried(session),
            ask_id: id.to_string(),
            choice,
        }));
        self.browser_asks_published(session).await
    }

    /// The Job's row as it reads now, published, so Needs you shows the ask or no longer does.
    async fn browser_asks_published(&self, session: &str) -> Result<ipc::SessionRecord, Refusal> {
        let record = {
            let store = self.store().lock().await;
            let kept = store.session(session).map_err(|why| self.ledger_fault(why))?.ok_or_else(|| self.settled_already())?;
            self.ledger_row(&store, &kept)?
        };
        self.publish(ipc::Event::SessionChanged(record.clone()));
        Ok(record)
    }

    /// The Job's page asks as Needs you items, while a client holds `/events` open. **With none, the
    /// tabs that raised them are gone, so they are dropped for good** and a Bridge that opens next
    /// starts with none.
    pub(crate) fn browser_ask_items(&self, session: &str) -> Vec<ipc::WaitingItem> {
        self.job_browser().asks_of(session, self.events().watching() > 0).into_iter().map(page_ask_item).collect()
    }
}

/// A page's ask as the Job's Needs you lists it.
fn page_ask_item(ask: PageAsk) -> ipc::WaitingItem {
    let id = format!("{ASK_ITEM}{}", ask.id);
    ipc::WaitingItem {
        act: Some(ipc::WaitingAct { kind: ipc::WaitingActKind::Answer, target: id.clone() }),
        id,
        text: ask.text,
        since: ipc::Instant::carried(&ask.since),
        source: ipc::WaitingSource::BrowserAsk,
        options: ask.options,
        steps: Vec::new(),
        place: None,
        confirmed: false,
        holders: Vec::new(),
        creating_since: None,
        create_refused: None,
        operator_read: None,
    }
}

/// `target` with its role checked, and every target it is looked for within.
fn aimed(target: WalkTarget) -> Result<WalkTarget, String> {
    if target.role.trim().is_empty() {
        return Err(String::from("a target names a role, such as button, link or textbox, or text"));
    }
    if let Some(within) = &target.within {
        aimed((**within).clone())?;
    }
    Ok(target)
}

/// Why `done` cannot answer `act`, where it cannot.
fn unfit(act: &BrowserAct, done: &Done) -> Option<String> {
    let Ok(answer) = done else { return None };
    match (act, answer) {
        (BrowserAct::Screenshot { path }, BrowserAnswer::Screenshot { .. }) => {
            let written = std::fs::metadata(path).is_ok_and(|file| file.len() > 0);
            (!written).then(|| format!("no screenshot is at {path}: write the PNG there, then answer"))
        }
        (BrowserAct::Evaluate { .. }, BrowserAnswer::Evaluated { .. }) => None,
        (BrowserAct::Screenshot { .. }, _) => Some(String::from("a screenshot is answered with kind screenshot")),
        (BrowserAct::Evaluate { .. }, _) => Some(String::from("an evaluate is answered with kind evaluated")),
        (_, BrowserAnswer::Page { .. }) => None,
        _ => Some(String::from("this act is answered with kind page")),
    }
}

/// A screenshot's answer with the act's path, which is the one the agent reads.
fn with_path(answer: BrowserAnswer, act: &BrowserAct) -> BrowserAnswer {
    match (answer, act) {
        (BrowserAnswer::Screenshot { url, title, .. }, BrowserAct::Screenshot { path }) => {
            BrowserAnswer::Screenshot { url, title, path: path.clone() }
        }
        (answer, _) => answer,
    }
}

/// `answer` held inside the door: a snapshot and a value cut at their bounds, saying so.
fn cut(answer: BrowserAnswer) -> BrowserAnswer {
    match answer {
        BrowserAnswer::Page { url, title, snapshot, .. } => {
            let (snapshot, truncated) = first(snapshot, MOST_SNAPSHOT);
            BrowserAnswer::Page { url, title, snapshot, truncated }
        }
        BrowserAnswer::Evaluated { url, title, result, .. } => {
            let (result, truncated) = match result {
                Some(result) => {
                    let (result, truncated) = first(result, MOST_RESULT);
                    (Some(result), truncated)
                }
                None => (None, false),
            };
            BrowserAnswer::Evaluated { url, title, result, truncated }
        }
        shot => shot,
    }
}

/// The first `most` bytes of `text`, cut at a character, and whether anything was cut.
fn first(mut text: String, most: usize) -> (String, bool) {
    if text.len() <= most {
        return (text, false);
    }
    let mut end = most;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}

/// What a target reads as in a sentence.
fn described(target: &WalkTarget) -> String {
    let name = target.name.as_deref().map(str::trim).filter(|name| !name.is_empty());
    match (target.role.trim(), name) {
        ("text", Some(name)) => format!("the words \"{name}\""),
        ("text", None) => String::from("any words"),
        (role, Some(name)) => format!("a {role} named \"{name}\""),
        (role, None) => format!("a {role}"),
    }
}

fn not_found(act: &BrowserAct) -> String {
    let target = match act {
        BrowserAct::Click { target } | BrowserAct::Type { target, .. } => Some(target),
        BrowserAct::WaitFor { target, .. } => target.as_ref(),
        _ => None,
    };
    match target {
        Some(target) => format!("Nothing on the page is {}. browser_snapshot shows what is there.", described(target)),
        None => String::from("The Job browser found nothing to act on. browser_snapshot shows what is there."),
    }
}

fn timed_out(act: &BrowserAct) -> String {
    match act {
        BrowserAct::WaitFor { text: Some(text), timeout_ms, .. } => format!("\"{text}\" did not appear on the page within {timeout_ms} ms."),
        BrowserAct::WaitFor { target: Some(target), timeout_ms, .. } => {
            format!("{} did not appear on the page within {timeout_ms} ms.", capitalised(&described(target)))
        }
        BrowserAct::Open { url } => format!("{url} did not finish loading in the Job browser."),
        _ => String::from("The Job browser ran out of time on that act."),
    }
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
