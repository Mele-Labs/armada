//! The Job browser's calls, each held until Bridge answers it. **Never written down**: a held call
//! is an agent's open HTTP request, which a restart cuts anyway.
//! `docs/practices/protocol/the-job-browser.md`.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use adapter_traits::{AgentHarness, Delivery, Vcs, WorkProduct};
use api::Refusal;
use ipc::{AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserError, BrowserRequest, BrowserRequestAnswered, SessionId, WalkTarget, WireError};
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
const OWNER_DRIVING: &str = "fleet.browser_owner_driving";
const TARGET_NOT_FOUND: &str = "fleet.browser_target_not_found";
const TIMED_OUT: &str = "fleet.browser_timed_out";
const FAILED: &str = "fleet.browser_failed";
const UNANSWERED: &str = "fleet.browser_unanswered";
const NO_SUCH_REQUEST: &str = "fleet.no_such_browser_request";
const WITHDRAWN: &str = "fleet.browser_request_withdrawn";
const ANSWERED: &str = "fleet.browser_request_answered";
const ANSWER_REFUSED: &str = "fleet.browser_answer_refused";

pub(crate) const OWNER_DRIVING_SAID: &str = "The owner is driving the Job browser. Ask him to hand it back, or wait.";
pub(crate) const UNANSWERED_SAID: &str = "Bridge did not answer. Is the Job browser open?";

type Done = Result<BrowserAnswer, BrowserError>;

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
    mint: UlidMint,
    stopping: AtomicBool,
    within_ms: AtomicU64,
}

impl Default for Browsers {
    fn default() -> Browsers {
        Browsers {
            held: Mutex::default(),
            ended: Mutex::default(),
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
            Err(BrowserError::OwnerDriving) => Err(stuck(OWNER_DRIVING, String::from(OWNER_DRIVING_SAID))),
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
