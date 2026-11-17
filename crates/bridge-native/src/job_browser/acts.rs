//! One agent act on a Job's page, done and answered: Classic's `job-browser-acts.ts`, ported.
//!
//! **Fleet holds the agent's call until Bridge posts `answer_browser_request`, so every request is answered once**, with an
//! answer or an error, whatever happens on the way: a panic or a WebKit error is answered as `failed`, and the act's mark on
//! the page ends before the answer is posted. `docs/practices/protocol/the-job-browser.md`.
//!
//! What an act does to the page is `Page`, which the real page implements (`live.rs`) and a test fakes.

use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use ipc::{AnswerBrowserRequest, BrowserAct, BrowserAnswer, BrowserError, BrowserRequest, WalkTarget};

/// An address finishes loading within this, or the agent is told it did not: Fleet holds the call 20 seconds, and the snapshot
/// after it takes up to 7 (`LOAD_MS`).
pub const LOAD: Duration = Duration::from_millis(12_000);
/// How long a target is waited for before a click or a typing says it is not there (Classic's `PATIENCE_MS`).
pub const PATIENCE: Duration = Duration::from_millis(5_000);
/// A beat after an input, so the page has drawn what it did before it is read (`SETTLE_MS`).
pub const SETTLE: Duration = Duration::from_millis(300);
/// Past Fleet's cuts (48 KiB and 32 KiB), so Fleet still sees an answer that was cut and says so.
pub const MOST_SNAPSHOT: usize = 64 * 1024;
pub const MOST_RESULT: usize = 64 * 1024;

pub const CLOSED: &str = "The Job browser is closed. Call browser_open first.";

/// How a page's load ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loaded {
    Loaded,
    Failed(String),
    TimedOut,
}

/// What an act does to the Job's page. An `Err` is a throw: the act is answered `failed` with its words.
#[allow(async_fn_in_trait)]
pub trait Page {
    fn url(&self) -> String;
    fn title(&self) -> String;
    /// Load an address and wait for it.
    async fn load(&self, url: &str, patience: Duration) -> Loaded;
    /// Find the target, scroll it into view and hold it for `press` and `fill`. False: not on the page within `patience`.
    async fn find(&self, target: &WalkTarget, patience: Duration) -> Result<bool, String>;
    /// Press the held element. False: it left the page.
    async fn press(&self) -> Result<bool, String>;
    /// Give the held element the keyboard and type `text` over what is in it; then Enter where `submit`. False: it left the page.
    async fn fill(&self, text: &str, submit: bool) -> Result<bool, String>;
    /// A beat, so the page has drawn what the input did before it is read.
    async fn settle(&self);
    async fn snapshot(&self) -> Result<String, String>;
    /// The page as a PNG at `path`. False: nothing could be taken or written.
    async fn capture(&self, path: &str) -> bool;
    /// `expression` in a world of the page's own: its value as `JSON.stringify` writes it, or none where it has none.
    async fn evaluate(&self, expression: &str) -> Result<Option<String>, String>;
}

/// The app's half: which page a Job's acts reach, the mark an act is running, a page opened for a Job with none, and the answer.
#[allow(async_fn_in_trait)]
pub trait Host {
    type Page: Page;
    /// What marks the act on its page while it is held, and ends the mark when dropped.
    type Mark;
    fn page(&self, job: &str) -> Option<Self::Page>;
    fn acting(&self, job: &str, sentence: String) -> Self::Mark;
    /// **An agent's `open` on a Job with no Browser tab opens a page for it**, which the first Browser tab takes over.
    fn open_closed(&self, job: &str, url: &str) -> Self::Page;
    async fn answer(&self, body: AnswerBrowserRequest);
}

type Reply = Result<BrowserAnswer, BrowserError>;

fn failed(why: impl Into<String>) -> Reply {
    Err(BrowserError::Failed { why: why.into() })
}

fn not_found() -> Reply {
    Err(BrowserError::NotFound)
}

/// The host of an address, else the address as given (`hostOf`).
fn host_of(url: &str) -> String {
    let Some((_, rest)) = url.split_once("://") else { return url.to_string() };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    if host.is_empty() {
        url.to_string()
    } else {
        host.to_lowercase()
    }
}

/// The words a target is known by: its name, else its role.
fn named(target: &WalkTarget) -> &str {
    target.name.as_deref().unwrap_or(&target.role)
}

/// The sentence for an act that is running: what the agent's mark says (`describeAct`).
pub fn describe_act(act: &BrowserAct) -> String {
    match act {
        BrowserAct::Open { url } => format!("Opening {}", host_of(url)),
        BrowserAct::Click { target } => format!("Clicking {}", named(target)),
        BrowserAct::Type { target, .. } => format!("Typing in {}", named(target)),
        BrowserAct::Snapshot => "Reading the page".into(),
        BrowserAct::Screenshot { .. } => "Taking a picture of the page".into(),
        BrowserAct::WaitFor { text, target, .. } => match text.as_deref().or(target.as_ref().map(named)) {
            Some(what) => format!("Waiting for {what}"),
            None => "Waiting for the page".into(),
        },
        BrowserAct::Evaluate { .. } => "Running a script".into(),
    }
}

/// The Job's page, the one its agent's acts reach: the first Browser tab in pane order that is shown, else the one shown last, else the
/// first (`jobsPage`). `tabs` is each tab's id and whether it is shown, in pane order; the answer is an index into it.
pub fn jobs_page<T: AsRef<str>>(tabs: &[(T, bool)], last_shown: Option<&str>) -> Option<usize> {
    tabs.iter()
        .position(|(_, shown)| *shown)
        .or_else(|| last_shown.and_then(|last| tabs.iter().position(|(id, _)| id.as_ref() == last)))
        .or(if tabs.is_empty() { None } else { Some(0) })
}

/// The source an agent's expression runs as: awaited, and its value written by `JSON.stringify` inside the page's world, cut at the
/// most a result may be (`evaluateSource`).
pub fn evaluate_source(expression: &str) -> String {
    format!("(async () => {{ const value = await (\n{expression}\n); const said = JSON.stringify(value); return said === undefined ? undefined : said.slice(0, {MOST_RESULT}); }})()")
}

/// Whether an address is one the Job browser opens: http or https, in any case (`/^https?:\/\//i`).
pub fn is_web(url: &str) -> bool {
    let lower = url.get(..8).map_or_else(|| url.to_lowercase(), str::to_lowercase);
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Whether a picture may be written to `path`: absolute, and a PNG.
pub fn png_path(path: &str) -> bool {
    path.starts_with('/') && path.ends_with(".png")
}

async fn page_of<P: Page>(page: &P) -> Reply {
    match page.snapshot().await {
        Ok(snapshot) => Ok(BrowserAnswer::Page { url: page.url(), title: page.title(), snapshot, truncated: false }),
        Err(why) => failed(why),
    }
}

async fn loaded<P: Page>(page: &P, url: &str) -> Reply {
    if !is_web(url) {
        return failed(format!("{url} is not an http or https address."));
    }
    match page.load(url, LOAD).await {
        Loaded::TimedOut => Err(BrowserError::TimedOut),
        Loaded::Failed(why) => failed(format!("{url} did not load: {why}")),
        Loaded::Loaded => page_of(page).await,
    }
}

async fn done<H: Host>(host: &H, job: &str, act: BrowserAct) -> Reply {
    let Some(page) = host.page(job) else {
        return match act {
            BrowserAct::Open { url } => loaded(&host.open_closed(job, &url), &url).await,
            _ => failed(CLOSED),
        };
    };
    match act {
        BrowserAct::Open { url } => loaded(&page, &url).await,
        BrowserAct::Click { target } => {
            match page.find(&target, PATIENCE).await {
                Ok(true) => {}
                Ok(false) => return not_found(),
                Err(why) => return failed(why),
            }
            match page.press().await {
                Ok(true) => {}
                Ok(false) => return not_found(),
                Err(why) => return failed(why),
            }
            page.settle().await;
            page_of(&page).await
        }
        BrowserAct::Type { target, text, submit } => {
            match page.find(&target, PATIENCE).await {
                Ok(true) => {}
                Ok(false) => return not_found(),
                Err(why) => return failed(why),
            }
            // Typed over, not into, as a walk's `type` is.
            match page.fill(&text, submit).await {
                Ok(true) => {}
                Ok(false) => return not_found(),
                Err(why) => return failed(why),
            }
            page.settle().await;
            page_of(&page).await
        }
        BrowserAct::Snapshot => page_of(&page).await,
        BrowserAct::Screenshot { path } => {
            if page.capture(&path).await {
                Ok(BrowserAnswer::Screenshot { url: page.url(), title: page.title(), path: String::new() })
            } else {
                failed("No picture of the page could be taken.")
            }
        }
        BrowserAct::WaitFor { text, target, timeout_ms } => {
            let target = target.or_else(|| text.map(|words| WalkTarget { role: "text".into(), name: Some(words), exact: None, within: None }));
            let Some(target) = target else { return failed("wait_for names neither text nor a target.") };
            match page.find(&target, Duration::from_millis(timeout_ms)).await {
                Ok(true) => page_of(&page).await,
                Ok(false) => Err(BrowserError::TimedOut),
                Err(why) => failed(why),
            }
        }
        BrowserAct::Evaluate { expression } => match page.evaluate(&expression).await {
            Ok(result) => Ok(BrowserAnswer::Evaluated { url: page.url(), title: page.title(), result, truncated: false }),
            Err(why) => failed(why),
        },
    }
}

/// A future that answers a panic in it as `Err`, with the panic's words: a poll that unwinds is the act's end, not the app's.
struct Guarded<F> {
    inner: Pin<Box<F>>,
}

impl<F: Future> Future for Guarded<F> {
    type Output = Result<F::Output, String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.inner.as_mut();
        match catch_unwind(AssertUnwindSafe(|| inner.poll(cx))) {
            Ok(Poll::Ready(out)) => Poll::Ready(Ok(out)),
            Ok(Poll::Pending) => Poll::Pending,
            Err(panic) => Poll::Ready(Err(panic
                .downcast_ref::<&str>()
                .map(|words| words.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "The Job browser stopped.".to_string()))),
        }
    }
}

/// Do the act and answer it. A throw or a panic on the way is answered as `failed`, so the agent's call never waits out Fleet's
/// hold. **The act is marked on the page while it runs** and unmarked as it answers, whatever became of it.
pub async fn act_on<H: Host>(host: &H, request: BrowserRequest) {
    let job = request.session_id.as_str().to_string();
    let mark = host.acting(&job, describe_act(&request.act));
    let reply = match (Guarded { inner: Box::pin(done(host, &job, request.act)) }).await {
        Ok(reply) => reply,
        Err(why) => failed(why),
    };
    drop(mark);
    let (answer, error) = match reply {
        Ok(answer) => (Some(answer), None),
        Err(error) => (None, Some(error)),
    };
    host.answer(AnswerBrowserRequest { request_id: request.request_id, answer, error }).await;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use ipc::SessionId;

    use super::*;

    fn target(role: &str, name: &str) -> WalkTarget {
        WalkTarget { role: role.into(), name: Some(name.into()), exact: None, within: None }
    }

    /// A page with a Save button and nothing else.
    #[derive(Clone)]
    struct Fake {
        log: Rc<RefCell<Vec<String>>>,
        snapshot: Rc<dyn Fn() -> Result<String, String>>,
        load: Loaded,
        evaluated: Result<Option<String>, String>,
        captured: bool,
    }

    fn fake() -> Fake {
        Fake { log: Rc::default(), snapshot: Rc::new(|| Ok("- button \"Save\"".into())), load: Loaded::Loaded, evaluated: Ok(Some("1".into())), captured: true }
    }

    impl Page for Fake {
        fn url(&self) -> String {
            "http://localhost:5173/checkout".into()
        }
        fn title(&self) -> String {
            "Checkout".into()
        }
        async fn load(&self, url: &str, patience: Duration) -> Loaded {
            self.log.borrow_mut().push(format!("load {url} within {}", patience.as_millis()));
            self.load.clone()
        }
        async fn find(&self, target: &WalkTarget, patience: Duration) -> Result<bool, String> {
            self.log.borrow_mut().push(format!("find {} {} within {}", target.role, target.name.clone().unwrap_or_default(), patience.as_millis()));
            Ok(target.role == "button" && target.name.as_deref() == Some("Save") || target.role == "text")
        }
        async fn press(&self) -> Result<bool, String> {
            self.log.borrow_mut().push("press".into());
            Ok(true)
        }
        async fn fill(&self, text: &str, submit: bool) -> Result<bool, String> {
            self.log.borrow_mut().push(format!("fill {text} submit {submit}"));
            Ok(true)
        }
        async fn settle(&self) {
            self.log.borrow_mut().push("settle".into());
        }
        async fn snapshot(&self) -> Result<String, String> {
            (self.snapshot)()
        }
        async fn capture(&self, path: &str) -> bool {
            self.log.borrow_mut().push(format!("capture {path}"));
            self.captured
        }
        async fn evaluate(&self, expression: &str) -> Result<Option<String>, String> {
            self.log.borrow_mut().push(format!("evaluate {expression}"));
            self.evaluated.clone()
        }
    }

    /// Ends its mark when dropped, as the app's does.
    struct Mark {
        marks: Rc<RefCell<Vec<String>>>,
        sentence: String,
    }

    impl Drop for Mark {
        fn drop(&mut self) {
            self.marks.borrow_mut().push(format!("end {}", self.sentence));
        }
    }

    struct FakeHost {
        page: Option<Fake>,
        fresh: Fake,
        answered: Rc<RefCell<Vec<AnswerBrowserRequest>>>,
        opened: Rc<RefCell<Vec<String>>>,
        marks: Rc<RefCell<Vec<String>>>,
    }

    fn host(page: Option<Fake>) -> FakeHost {
        FakeHost { page, fresh: fake(), answered: Rc::default(), opened: Rc::default(), marks: Rc::default() }
    }

    impl Host for FakeHost {
        type Page = Fake;
        type Mark = Mark;
        fn page(&self, _job: &str) -> Option<Fake> {
            self.page.clone()
        }
        fn acting(&self, _job: &str, sentence: String) -> Mark {
            self.marks.borrow_mut().push(format!("start {sentence}"));
            Mark { marks: self.marks.clone(), sentence }
        }
        fn open_closed(&self, _job: &str, url: &str) -> Fake {
            self.opened.borrow_mut().push(url.into());
            self.fresh.clone()
        }
        async fn answer(&self, body: AnswerBrowserRequest) {
            self.answered.borrow_mut().push(body);
        }
    }

    fn asked(act: BrowserAct) -> BrowserRequest {
        BrowserRequest { session_id: SessionId::carried("01JOB".to_string()), request_id: "browser-01".into(), act }
    }

    fn run(host: &FakeHost, act: BrowserAct) {
        gpui::block_on(act_on(host, asked(act)));
    }

    fn page_answer(snapshot: &str) -> AnswerBrowserRequest {
        AnswerBrowserRequest {
            request_id: "browser-01".into(),
            answer: Some(BrowserAnswer::Page { url: "http://localhost:5173/checkout".into(), title: "Checkout".into(), snapshot: snapshot.into(), truncated: false }),
            error: None,
        }
    }

    fn error_answer(error: BrowserError) -> AnswerBrowserRequest {
        AnswerBrowserRequest { request_id: "browser-01".into(), answer: None, error: Some(error) }
    }

    #[test]
    fn a_click_finds_the_target_presses_it_settles_and_answers_the_page() {
        let page = fake();
        let host = host(Some(page.clone()));
        run(&host, BrowserAct::Click { target: target("button", "Save") });
        assert_eq!(*page.log.borrow(), ["find button Save within 5000", "press", "settle"]);
        assert_eq!(*host.answered.borrow(), [page_answer("- button \"Save\"")]);
    }

    #[test]
    fn the_act_is_marked_while_it_runs_and_unmarked_once_it_is_over() {
        let mut page = fake();
        let host = host(None);
        let marks = host.marks.clone();
        let seen: Rc<RefCell<Vec<Vec<String>>>> = Rc::default();
        let during = seen.clone();
        page.snapshot = Rc::new(move || {
            during.borrow_mut().push(marks.borrow().clone());
            Ok("page".into())
        });
        let host = FakeHost { page: Some(page), ..host };
        run(&host, BrowserAct::Snapshot);
        assert_eq!(*seen.borrow(), [vec!["start Reading the page".to_string()]]);
        assert_eq!(*host.marks.borrow(), ["start Reading the page", "end Reading the page"]);
        assert_eq!(host.answered.borrow().len(), 1);
    }

    #[test]
    fn the_mark_ends_where_the_act_throws_and_where_the_page_is_closed() {
        let mut page = fake();
        page.snapshot = Rc::new(|| Err("the page crashed".into()));
        let threw = host(Some(page));
        run(&threw, BrowserAct::Snapshot);
        assert_eq!(*threw.marks.borrow(), ["start Reading the page", "end Reading the page"]);
        let closed = host(None);
        run(&closed, BrowserAct::Click { target: target("button", "Save") });
        assert_eq!(*closed.marks.borrow(), ["start Clicking Save", "end Clicking Save"]);
    }

    #[test]
    fn nothing_found_is_not_found_and_the_page_is_pressed_nowhere() {
        let page = fake();
        let host = host(Some(page.clone()));
        run(&host, BrowserAct::Click { target: target("button", "Delete") });
        assert_eq!(*page.log.borrow(), ["find button Delete within 5000"]);
        assert_eq!(*host.answered.borrow(), [error_answer(BrowserError::NotFound)]);
    }

    #[test]
    fn an_open_on_a_job_with_no_browser_tab_opens_a_page_for_it_and_answers_its_page() {
        let host = host(None);
        run(&host, BrowserAct::Open { url: "http://localhost:5173/cart".into() });
        assert_eq!(*host.opened.borrow(), ["http://localhost:5173/cart"]);
        assert_eq!(*host.fresh.log.borrow(), ["load http://localhost:5173/cart within 12000"]);
        assert_eq!(*host.answered.borrow(), [page_answer("- button \"Save\"")]);
    }

    #[test]
    fn any_other_act_on_a_closed_job_browser_is_refused_in_words_that_name_the_way_out() {
        let host = host(None);
        run(&host, BrowserAct::Snapshot);
        assert!(host.opened.borrow().is_empty());
        assert_eq!(*host.answered.borrow(), [error_answer(BrowserError::Failed { why: CLOSED.into() })]);
    }

    #[test]
    fn a_throw_on_the_way_is_answered_as_failed_so_the_agents_call_is_never_left_held() {
        let mut page = fake();
        page.snapshot = Rc::new(|| Err("The page went away.".into()));
        let host = host(Some(page));
        run(&host, BrowserAct::Snapshot);
        assert_eq!(*host.answered.borrow(), [error_answer(BrowserError::Failed { why: "The page went away.".into() })]);
    }

    #[test]
    fn a_panic_on_the_way_is_answered_as_failed_and_the_mark_still_ends() {
        let mut page = fake();
        page.snapshot = Rc::new(|| panic!("the finder fell over"));
        let host = host(Some(page));
        run(&host, BrowserAct::Snapshot);
        assert_eq!(*host.answered.borrow(), [error_answer(BrowserError::Failed { why: "the finder fell over".into() })]);
        assert_eq!(*host.marks.borrow(), ["start Reading the page", "end Reading the page"]);
    }

    #[test]
    fn an_address_that_is_not_http_or_https_is_refused_and_never_loaded() {
        let page = fake();
        let host = host(Some(page.clone()));
        run(&host, BrowserAct::Open { url: "file:///etc/passwd".into() });
        assert!(page.log.borrow().is_empty());
        assert_eq!(*host.answered.borrow(), [error_answer(BrowserError::Failed { why: "file:///etc/passwd is not an http or https address.".into() })]);
    }

    #[test]
    fn a_load_that_ran_out_is_timed_out_and_one_that_failed_says_why() {
        let mut slow = fake();
        slow.load = Loaded::TimedOut;
        let host_slow = host(Some(slow));
        run(&host_slow, BrowserAct::Open { url: "https://example.com/".into() });
        assert_eq!(*host_slow.answered.borrow(), [error_answer(BrowserError::TimedOut)]);

        let mut broken = fake();
        broken.load = Loaded::Failed("A server with the specified hostname could not be found.".into());
        let host_broken = host(Some(broken));
        run(&host_broken, BrowserAct::Open { url: "https://nowhere.invalid/".into() });
        assert_eq!(
            *host_broken.answered.borrow(),
            [error_answer(BrowserError::Failed { why: "https://nowhere.invalid/ did not load: A server with the specified hostname could not be found.".into() })]
        );
    }

    #[test]
    fn typing_goes_over_what_is_there_and_submits_only_where_asked() {
        let page = fake();
        let host = host(Some(page.clone()));
        run(&host, BrowserAct::Type { target: target("button", "Save"), text: "hello".into(), submit: true });
        assert_eq!(*page.log.borrow(), ["find button Save within 5000", "fill hello submit true", "settle"]);
        assert_eq!(*host.answered.borrow(), [page_answer("- button \"Save\"")]);
    }

    #[test]
    fn wait_for_words_looks_for_text_and_runs_out_as_timed_out_after_the_time_fleet_held() {
        let page = fake();
        let seen = host(Some(page.clone()));
        run(&seen, BrowserAct::WaitFor { text: Some("Saved".into()), target: None, timeout_ms: 7000 });
        assert_eq!(*page.log.borrow(), ["find text Saved within 7000"]);
        assert_eq!(*seen.answered.borrow(), [page_answer("- button \"Save\"")]);

        let gone = host(Some(fake()));
        run(&gone, BrowserAct::WaitFor { text: None, target: Some(target("button", "Delete")), timeout_ms: 100 });
        assert_eq!(*gone.answered.borrow(), [error_answer(BrowserError::TimedOut)]);

        let neither = host(Some(fake()));
        run(&neither, BrowserAct::WaitFor { text: None, target: None, timeout_ms: 100 });
        assert_eq!(*neither.answered.borrow(), [error_answer(BrowserError::Failed { why: "wait_for names neither text nor a target.".into() })]);
    }

    #[test]
    fn a_screenshot_is_answered_with_the_page_and_no_path_and_fails_where_none_could_be_taken() {
        let page = fake();
        let host_ok = host(Some(page.clone()));
        run(&host_ok, BrowserAct::Screenshot { path: "/tmp/scratch/browser/browser-01.png".into() });
        assert_eq!(*page.log.borrow(), ["capture /tmp/scratch/browser/browser-01.png"]);
        assert_eq!(
            *host_ok.answered.borrow(),
            [AnswerBrowserRequest {
                request_id: "browser-01".into(),
                answer: Some(BrowserAnswer::Screenshot { url: "http://localhost:5173/checkout".into(), title: "Checkout".into(), path: String::new() }),
                error: None,
            }]
        );
        let mut none = fake();
        none.captured = false;
        let host_none = host(Some(none));
        run(&host_none, BrowserAct::Screenshot { path: "/tmp/x.png".into() });
        assert_eq!(*host_none.answered.borrow(), [error_answer(BrowserError::Failed { why: "No picture of the page could be taken.".into() })]);
    }

    #[test]
    fn an_evaluate_answers_its_json_or_nothing_where_the_value_has_none_or_the_throw_where_it_threw() {
        let host_value = host(Some(fake()));
        run(&host_value, BrowserAct::Evaluate { expression: "1".into() });
        assert_eq!(
            *host_value.answered.borrow(),
            [AnswerBrowserRequest {
                request_id: "browser-01".into(),
                answer: Some(BrowserAnswer::Evaluated { url: "http://localhost:5173/checkout".into(), title: "Checkout".into(), result: Some("1".into()), truncated: false }),
                error: None,
            }]
        );
        let mut empty = fake();
        empty.evaluated = Ok(None);
        let host_empty = host(Some(empty));
        run(&host_empty, BrowserAct::Evaluate { expression: "undefined".into() });
        assert!(matches!(host_empty.answered.borrow()[0].answer, Some(BrowserAnswer::Evaluated { result: None, .. })));
        let mut threw = fake();
        threw.evaluated = Err("boom is not defined".into());
        let host_threw = host(Some(threw));
        run(&host_threw, BrowserAct::Evaluate { expression: "boom".into() });
        assert_eq!(*host_threw.answered.borrow(), [error_answer(BrowserError::Failed { why: "boom is not defined".into() })]);
    }

    #[test]
    fn each_act_says_what_it_is_doing_in_one_sentence() {
        let said = |act: BrowserAct| describe_act(&act);
        assert_eq!(said(BrowserAct::Open { url: "https://shop.example.com:8443/cart?x=1".into() }), "Opening shop.example.com:8443");
        assert_eq!(said(BrowserAct::Open { url: "not an address".into() }), "Opening not an address");
        assert_eq!(said(BrowserAct::Click { target: target("button", "Save") }), "Clicking Save");
        assert_eq!(said(BrowserAct::Click { target: WalkTarget { role: "button".into(), name: None, exact: None, within: None } }), "Clicking button");
        assert_eq!(said(BrowserAct::Type { target: target("textbox", "Email"), text: "a".into(), submit: false }), "Typing in Email");
        assert_eq!(said(BrowserAct::Snapshot), "Reading the page");
        assert_eq!(said(BrowserAct::Screenshot { path: String::new() }), "Taking a picture of the page");
        assert_eq!(said(BrowserAct::WaitFor { text: Some("Saved".into()), target: None, timeout_ms: 1 }), "Waiting for Saved");
        assert_eq!(said(BrowserAct::WaitFor { text: None, target: Some(target("dialog", "Confirm")), timeout_ms: 1 }), "Waiting for Confirm");
        assert_eq!(said(BrowserAct::WaitFor { text: None, target: None, timeout_ms: 1 }), "Waiting for the page");
        assert_eq!(said(BrowserAct::Evaluate { expression: "1".into() }), "Running a script");
    }

    #[test]
    fn the_jobs_page_is_the_first_one_shown_else_the_one_shown_last_else_the_first() {
        let tabs = [("t1", false), ("t2", true), ("t3", true)];
        assert_eq!(jobs_page(&tabs, Some("t1")), Some(1), "what he is looking at, first in pane order");
        let none_shown = [("t1", false), ("t2", false), ("t3", false)];
        assert_eq!(jobs_page(&none_shown, Some("t3")), Some(2), "the one shown last");
        assert_eq!(jobs_page(&none_shown, Some("gone")), Some(0), "else the first");
        assert_eq!(jobs_page(&none_shown, None), Some(0));
        assert_eq!(jobs_page::<&str>(&[], Some("t1")), None);
    }

    #[test]
    fn an_expression_runs_awaited_and_its_value_is_written_as_json_and_cut() {
        let source = evaluate_source("document.title");
        assert!(source.starts_with("(async () => { const value = await (\ndocument.title\n);"), "{source}");
        assert!(source.contains("JSON.stringify(value)"));
        assert!(source.contains("said.slice(0, 65536)"));
        assert!(source.ends_with("})()"));
    }

    #[test]
    fn only_http_and_https_addresses_open_whatever_their_case() {
        assert!(is_web("http://localhost:5173"));
        assert!(is_web("HTTPS://Example.com"));
        assert!(!is_web("file:///tmp/x"));
        assert!(!is_web("javascript:alert(1)"));
        assert!(!is_web("example.com"));
        assert!(!is_web("http:/x"));
    }

    #[test]
    fn a_picture_is_written_only_to_an_absolute_png_path() {
        assert!(png_path("/tmp/a/browser-01.png"));
        assert!(!png_path("browser-01.png"));
        assert!(!png_path("/tmp/a.jpg"));
    }
}
