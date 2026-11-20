//! Reviews on Fleet: the inbox, each pull request's page and the runs a review asks for, read over the
//! app's one Fleet connection. `map` turns Fleet's answers into the screen's view model; everything
//! here asks, waits and lands, and works nothing out that Fleet's answers do not carry.

use std::collections::{HashMap, HashSet};

use gpui::{Context, Entity, Subscription, Task, Window};
use ipc::{
    PullRequestCompare, PullRequestFilePatch, PullRequestFilePatches, PullRequestNote, PullRequestNotes, PullRequestPage, PullReviewHolder, PullReviewLists, ReviewRun, ReviewRunKind,
    ReviewRuns, ReviewTools,
};
use serde::de::{DeserializeOwned, IgnoredAny};

use super::model::{self, Critique, Verdict};
use super::ReviewsScreen;
use crate::fleet::{FleetConnection, ReviewListsMoved, ReviewRunsMoved};
use crate::job_view::pane_tree::Arrangement;
use crate::job_view::wire::encoded;

mod kept;
pub(super) mod map;

pub(super) use map::Find;

/// The explanation Fleet ships for what changed since he looked: one paragraph.
pub(super) const WHAT_CHANGED: &str = "what-changed";

/// What one pull request's row said, kept so its page can be read and named.
#[derive(Clone, Debug)]
pub(super) struct Row {
    pub manifest: String,
    pub repository: String,
    pub title: String,
    pub branch: String,
    pub author: String,
    pub holder: Option<PullReviewHolder>,
    /// Where it is on the forge.
    pub address: String,
    pub draft: bool,
    /// The head the lists read, whole: what Mark as seen in the inbox keeps.
    pub head: String,
}

/// One pull request as Fleet last answered for it.
pub(super) struct Pull {
    pub row: Row,
    pub facts: map::Facts,
    pub notes: Vec<PullRequestNote>,
    pub finds: map::Finds,
}

/// The live half of the screen: the connection, what each pull request's reads answered, and the
/// asks in flight.
pub(super) struct Live {
    fleet: Entity<FleetConnection>,
    pub pulls: HashMap<u64, Pull>,
    /// Review tools by manifest, read once each.
    tools: HashMap<String, Vec<model::ReviewTool>>,
    /// Why the inbox could not be read, until a read lands.
    pub inbox_said: Option<String>,
    /// Over rows kept from an earlier read, while the forge is rate-limiting us.
    pub inbox_banner: Option<String>,
    /// Why Fleet refused the last act on a row of the inbox, until the next act or read.
    act_said: Option<String>,
    /// The banner is the forge's rate limit, so the inbox reads again by itself when it lifts.
    banner_limited: bool,
    /// When the lists on screen were read, as Fleet said it, for the age in the header.
    fetched_at: Option<String>,
    /// Why a review was refused, by pull request, until he sends again.
    pub write_said: HashMap<u64, String>,
    pub writing: bool,
    /// Pull requests whose Find problems runs the overview once its change sets land.
    chained: HashSet<u64>,
    layouts_read: bool,
    tasks: HashMap<String, Task<()>>,
    _subs: Vec<Subscription>,
}

#[derive(Clone, Copy)]
enum Verb {
    Get,
    Post,
}

/// Fleet's own reason for a refusal, as a sentence he reads.
fn refusal(status: u16, bytes: &[u8]) -> String {
    ipc::decode::<ipc::WireError>("WireError", bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| map::refused_words(&error))
}

async fn fetch<T: DeserializeOwned>(port: u16, verb: Verb, path: &str, body: Option<String>, name: &'static str) -> Result<T, String> {
    let (status, bytes) = match verb {
        Verb::Get => crate::transport::get(port, path).await,
        Verb::Post => crate::transport::post(port, path, body.unwrap_or_default()).await,
    }?;
    if !(200..300).contains(&status) {
        return Err(refusal(status, &bytes));
    }
    ipc::decode::<T>(name, &bytes).map_err(|why| why.to_string())
}

fn runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())
}

fn runs_route(manifest: &str) -> String {
    format!("/review_runs/{}", encoded(manifest))
}

/// A repository's review tools, as the sheet lists them. Blocking: for a caller off the main thread, such as a review toast.
pub(crate) fn read_tools(port: u16, manifest: &str) -> Result<Vec<model::ReviewTool>, String> {
    let tools: ReviewTools = runtime()?.block_on(fetch(port, Verb::Get, &format!("/review_tools/{}", encoded(manifest)), None, "ReviewTools"))?;
    Ok(map::tools_of(&tools))
}

/// One run asked of Fleet, as `start_run` asks it. Blocking, as `read_tools`.
pub(crate) fn post_run(port: u16, manifest: &str, run: &ipc::StartReviewRun) -> Result<ReviewRun, String> {
    let body = ipc::encode(run).map_err(|why| why.to_string())?;
    runtime()?.block_on(fetch(port, Verb::Post, &runs_route(manifest), Some(body), "ReviewRun"))
}

/// Every run kept or going for one pull request. Blocking, as `read_tools`.
pub(crate) fn read_runs(port: u16, manifest: &str, number: u64) -> Result<Vec<ReviewRun>, String> {
    let runs: ReviewRuns = runtime()?.block_on(fetch(port, Verb::Get, &format!("{}/{number}", runs_route(manifest)), None, "ReviewRuns"))?;
    Ok(runs.runs)
}

/// One page read, mapped.
struct Landed {
    facts: map::Facts,
    notes: Vec<PullRequestNote>,
    tools: Option<ReviewTools>,
    page: model::Page,
    mapped: map::Mapped,
}

/// Everything one page read answered.
struct PageRead {
    detail: PullRequestPage,
    patches: Vec<PullRequestFilePatch>,
    compare: Option<PullRequestCompare>,
    runs: Vec<ReviewRun>,
    notes: Vec<PullRequestNote>,
    tools: Option<ReviewTools>,
}

impl ReviewsScreen {
    /// Reviews on Fleet: nothing drawn until the forge's lists are read.
    pub fn live(fleet: &Entity<FleetConnection>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut this = Self::mock(window, cx);
        this.inbox = model::Inbox::default();
        this.last_visit = None;
        this.drafts = Vec::new();
        this.sent_findings = HashMap::new();
        this.review_tools = Vec::new();
        let moved = cx.subscribe_in(fleet, window, |this, _, moved: &ReviewRunsMoved, window, cx| this.runs_moved(moved, window, cx));
        let lists_moved = cx.subscribe_in(fleet, window, |this, _, moved: &ReviewListsMoved, window, cx| this.lists_moved(moved, window, cx));
        this.live = Some(Live {
            fleet: fleet.clone(),
            pulls: HashMap::new(),
            tools: HashMap::new(),
            inbox_said: None,
            inbox_banner: None,
            act_said: None,
            banner_limited: false,
            fetched_at: None,
            write_said: HashMap::new(),
            writing: false,
            chained: HashSet::new(),
            layouts_read: false,
            tasks: HashMap::new(),
            _subs: vec![moved, lists_moved],
        });
        this
    }

    pub(super) fn is_live(&self) -> bool {
        self.live.is_some()
    }

    fn port(&self, cx: &gpui::App) -> Option<u16> {
        self.live.as_ref().and_then(|live| live.fleet.read(cx).held().port())
    }

    /// One ask of Fleet, its answer decoded off the main thread and handed to `done`. A later ask under
    /// the same `key` replaces it.
    #[allow(clippy::too_many_arguments)]
    fn call<T: DeserializeOwned + Send + 'static>(
        &mut self,
        key: String,
        verb: Verb,
        path: String,
        body: Option<String>,
        name: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, Result<T, String>, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let Some(port) = self.port(cx) else {
            done(self, Err("Fleet is not connected".into()), window, cx);
            return;
        };
        let task = cx.spawn_in(window, async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { runtime()?.block_on(fetch::<T>(port, verb, &path, body, name)) }).await;
            let _ = this.update_in(cx, |this, window, cx| done(this, answer, window, cx));
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert(key, task);
        }
    }

    /// `call` for an ask whose answer needs no window: one made as he leaves the screen, or of layouts.
    #[allow(clippy::too_many_arguments)]
    fn call_quiet<T: DeserializeOwned + Send + 'static>(
        &mut self,
        key: String,
        verb: Verb,
        path: String,
        body: Option<String>,
        name: &'static str,
        cx: &mut Context<Self>,
        done: impl FnOnce(&mut Self, Result<T, String>, &mut Context<Self>) + 'static,
    ) {
        let Some(port) = self.port(cx) else {
            done(self, Err("Fleet is not connected".into()), cx);
            return;
        };
        let task = cx.spawn(async move |this, cx| {
            let answer = cx.background_executor().spawn(async move { runtime()?.block_on(fetch::<T>(port, verb, &path, body, name)) }).await;
            let _ = this.update(cx, |this, cx| done(this, answer, cx));
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert(key, task);
        }
    }

    fn manifest_of(&self, number: u64) -> Option<String> {
        self.live.as_ref()?.pulls.get(&number).map(|pull| pull.row.manifest.clone())
    }

    // ---- The inbox ---------------------------------------------------------------------------------

    /// Refresh: the forge's lists again, and the page in front with them where its head moved. Fleet answers the
    /// lists it kept at once, with `refreshing`, and tells the fresh ones as `review_lists.changed` (`kept`).
    pub(super) fn live_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.inbox.refreshing = true;
        if !self.live.as_ref().is_some_and(|live| live.layouts_read) {
            self.live_read_layouts(cx);
        }
        self.call("lists".into(), Verb::Get, "/review_lists".into(), None, "PullReviewLists", window, cx, |this, answer: Result<PullReviewLists, String>, window, cx| {
            match answer {
                Ok(lists) => this.take_lists(lists, window, cx),
                Err(why) => {
                    this.inbox.refreshing = false;
                    this.refreshed = true;
                    if let Some(live) = this.live.as_mut() {
                        live.inbox_said = Some(why);
                    }
                }
            }
            this.read_again_when_limit_lifts(window, cx);
            cx.notify();
        });
        cx.notify();
    }

    /// While the forge is rate-limiting us the inbox reads again by itself: Fleet asks nothing until the hold lifts, so asking is cheap.
    fn read_again_when_limit_lifts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(live) = self.live.as_ref() else { return };
        let limited = live.banner_limited || live.inbox_said.as_deref().is_some_and(|said| said.ends_with(map::RATE_LIMIT_LIFTS));
        if !limited {
            return;
        }
        let wait = self.wait(super::LIMIT_REREAD, cx);
        let task = cx.spawn_in(window, async move |this, cx| {
            wait.await;
            let _ = this.update_in(cx, |this, window, cx| this.live_refresh(window, cx));
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert("limit-again".into(), task);
        }
    }

    /// A row pressed: its page as last read at once, read again behind it; never read, a skeleton until it lands.
    pub(super) fn live_open(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.pages.contains_key(&number) {
            self.waiting = None;
            self.show(number, window, cx);
        } else {
            self.waiting = Some(number);
        }
        self.live_read(number, window, cx);
        cx.notify();
    }

    /// A pull request asked of him that the inbox has not listed yet: held as its row would be, so its page reads.
    pub(super) fn live_hold_asked(&mut self, asked: &ipc::ReviewAsked) {
        let Some(live) = self.live.as_mut() else { return };
        live.pulls.entry(asked.number).or_insert_with(|| Pull {
            row: Row {
                manifest: asked.manifest_id.as_str().to_string(),
                repository: asked.repository.clone(),
                title: asked.title.clone(),
                branch: String::new(),
                author: asked.author.clone(),
                holder: None,
                address: asked.address.clone(),
                draft: false,
                head: String::new(),
            },
            facts: map::Facts::default(),
            notes: Vec::new(),
            finds: map::Finds::default(),
        });
    }

    // ---- A page ------------------------------------------------------------------------------------

    /// The page, its patches, the change since he looked, the runs kept for it, his notes and the
    /// repository's review tools, in one read off the main thread, mapped there too.
    pub(super) fn live_read(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.live.as_ref().and_then(|live| live.pulls.get(&number)).map(|pull| pull.row.clone()) else { return };
        let Some(port) = self.port(cx) else { return };
        let want_tools = !self.live.as_ref().is_some_and(|live| live.tools.contains_key(&row.manifest));
        let at = encoded(&row.manifest);
        let refused = self.live.as_ref().and_then(|live| live.pulls.get(&number)).map(|pull| pull.facts.refused.clone()).unwrap_or_default();
        let task = cx.spawn_in(window, async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn(async move {
                    let runtime = runtime()?;
                    let read = runtime.block_on(async {
                        let detail: PullRequestPage = fetch(port, Verb::Get, &format!("/pull_request_details/{at}/{number}"), None, "PullRequestPage").await?;
                        let patches: PullRequestFilePatches = fetch(port, Verb::Get, &format!("/pull_request_patches/{at}/{number}"), None, "PullRequestFilePatches").await?;
                        let compare = match (&detail.seen, &detail.head) {
                            (Some(seen), Some(head)) if &seen.head != head => {
                                let path = format!("/pull_request_compares/{at}/{number}?since={}&head={}", encoded(&seen.head), encoded(head));
                                fetch::<PullRequestCompare>(port, Verb::Get, &path, None, "PullRequestCompare").await.ok()
                            }
                            _ => None,
                        };
                        let runs = fetch::<ReviewRuns>(port, Verb::Get, &format!("/review_runs/{at}/{number}"), None, "ReviewRuns").await.map(|runs| runs.runs).unwrap_or_default();
                        let notes = fetch::<PullRequestNotes>(port, Verb::Get, &format!("/pull_request_notes/{at}/{number}"), None, "PullRequestNotes").await.map(|notes| notes.notes).unwrap_or_default();
                        let tools = if want_tools { fetch::<ReviewTools>(port, Verb::Get, &format!("/review_tools/{at}"), None, "ReviewTools").await.ok() } else { None };
                        Ok::<_, String>(PageRead { detail, patches: patches.files, compare, runs, notes, tools })
                    })?;
                    let page = map::page_of(number, &row, &read.detail, &read.patches, read.compare.as_ref(), &read.notes);
                    let facts = map::Facts {
                        head: read.detail.head.clone(),
                        seen: read.detail.seen.as_ref().map(|seen| seen.head.clone()),
                        gone: matches!(read.compare, Some(PullRequestCompare::BaseGone { .. })),
                        runs: read.runs,
                        patches: read.patches,
                        refused,
                    };
                    let mapped = map::from_runs(&page, &facts);
                    Ok::<_, String>(Landed { facts, notes: read.notes, tools: read.tools, page, mapped })
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| this.land_page(number, read, window, cx));
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert(format!("page:{number}"), task);
        }
    }

    fn land_page(&mut self, number: u64, read: Result<Landed, String>, window: &mut Window, cx: &mut Context<Self>) {
        let Landed { facts, notes, tools, mut page, mapped } = match read {
            Ok(read) => read,
            Err(why) => {
                // A page that could not be read stays as it was; one never read says why in its place.
                if self.waiting == Some(number) {
                    self.waiting = None;
                    if let Some(live) = self.live.as_mut() {
                        live.inbox_said = Some(format!("#{number} could not be read: {why}"));
                    }
                }
                cx.notify();
                return;
            }
        };
        let Some(live) = self.live.as_mut() else { return };
        if let Some(tools) = tools {
            live.tools.insert(page_manifest(&live.pulls, number), map::tools_of(&tools));
        }
        let Some(pull) = live.pulls.get_mut(&number) else { return };
        pull.facts = facts;
        pull.notes = notes;
        // What he did on the page stays: a critique he started that no run answers yet.
        if let Some(was) = self.pages.get(&number) {
            if matches!(mapped.critique, Critique::None) && matches!(was.critique, Critique::Running { .. }) {
                page.critique = was.critique.clone();
            }
        }
        self.pages.insert(number, page);
        self.apply(number, mapped, window, cx);
        // A page he is on takes its fresh parts where they are; one he waited for opens.
        if self.waiting == Some(number) {
            self.waiting = None;
            self.show(number, window, cx);
        } else if self.open == Some(number) {
            self.live_summary(number, window, cx);
        }
        cx.notify();
    }

    /// The review tools of the repository a pull request is in, as Fleet last read them, for Review this
    /// pull request. A page opens once its read lands, after `apply`, so opening it takes them too.
    pub(super) fn live_tools(&mut self, number: u64) {
        if let Some(tools) = self.live.as_ref().and_then(|live| live.tools.get(&page_manifest(&live.pulls, number))) {
            self.review_tools = tools.clone();
        }
    }

    /// What the runs come to lands on the page: the summary, the change sets and findings, the critique.
    fn apply(&mut self, number: u64, mapped: map::Mapped, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get_mut(&number) else { return };
        page.summary = mapped.summary;
        page.overview.sets = mapped.sets;
        page.overview.flags = mapped.flags;
        if !(matches!(mapped.critique, Critique::None) && matches!(page.critique, Critique::Running { .. })) {
            page.critique = mapped.critique;
        }
        let sets_done = mapped.finds.sets == Find::Done;
        let flags_absent = mapped.finds.flags == Find::NotRun;
        let sets_failed = matches!(mapped.finds.sets, Find::Failed(_));
        if let Some(pull) = self.live.as_mut().and_then(|live| live.pulls.get_mut(&number)) {
            pull.finds = mapped.finds;
        }
        for (rewrite, answer) in mapped.answers {
            self.rewritten.insert((number, rewrite), answer);
        }
        let chained = self.live.as_ref().is_some_and(|live| live.chained.contains(&number));
        if chained && (sets_done || sets_failed) {
            if let Some(live) = self.live.as_mut() {
                live.chained.remove(&number);
            }
            if sets_done && flags_absent {
                let again = self.live.as_ref().and_then(|live| live.pulls.get(&number)).is_some_and(|pull| pull.facts.kept_elsewhere(ReviewRunKind::Overview, ""));
                self.start_run(number, ReviewRunKind::Overview, "overview".into(), None, again, window, cx);
            }
        }
        if self.open == Some(number) {
            self.live_tools(number);
        }
        self.suggestion_boxes(number, window, cx);
        self.colour(number, cx);
        cx.notify();
    }

    /// Work out again what the runs come to, off the main thread, for a run that moved.
    fn remap(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get(&number).cloned() else { return };
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let facts = pull.facts.clone();
        let task = cx.spawn_in(window, async move |this, cx| {
            let mapped = cx.background_executor().spawn(async move { map::from_runs(&page, &facts) }).await;
            let _ = this.update_in(cx, |this, window, cx| this.apply(number, mapped, window, cx));
        });
        if let Some(live) = self.live.as_mut() {
            live.tasks.insert(format!("remap:{number}"), task);
        }
    }

    /// `review_run.changed`: each run lands on its pull request. A gap reads the open page's runs again.
    fn runs_moved(&mut self, moved: &ReviewRunsMoved, window: &mut Window, cx: &mut Context<Self>) {
        let mut touched: Vec<u64> = Vec::new();
        for run in &moved.runs {
            let Some(pull) = self.live.as_mut().and_then(|live| live.pulls.get_mut(&run.number)) else { continue };
            if pull.row.manifest != run.manifest_id.as_str() {
                continue;
            }
            land_run(&mut pull.facts.runs, run.clone());
            if !touched.contains(&run.number) {
                touched.push(run.number);
            }
        }
        for number in touched {
            if self.pages.contains_key(&number) {
                self.remap(number, window, cx);
            }
        }
        if moved.lost {
            if let Some(number) = self.open {
                self.live_reread_runs(number, window, cx);
            }
        }
    }

    fn live_reread_runs(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = self.manifest_of(number) else { return };
        self.call(format!("runs:{number}"), Verb::Get, format!("/review_runs/{}/{number}", encoded(&manifest)), None, "ReviewRuns", window, cx, move |this, answer: Result<ReviewRuns, String>, window, cx| {
            if let (Ok(runs), Some(pull)) = (answer, this.live.as_mut().and_then(|live| live.pulls.get_mut(&number))) {
                pull.facts.runs = runs.runs;
                this.remap(number, window, cx);
            }
        });
    }

    /// One run asked of Fleet. It answers the run at once, kept or queued; the rest arrives as `review_run.changed`.
    #[allow(clippy::too_many_arguments)]
    fn start_run(&mut self, number: u64, kind: ReviewRunKind, name: String, since: Option<String>, again: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(manifest) = self.manifest_of(number) else { return };
        let body = ipc::encode(&ipc::StartReviewRun { number, kind, name: name.clone(), again, place: None, finding: None, since }).ok();
        let key = format!("run:{number}:{kind:?}:{name}");
        self.call(key, Verb::Post, runs_route(&manifest), body, "ReviewRun", window, cx, move |this, answer: Result<ReviewRun, String>, window, cx| {
            let Some(pull) = this.live.as_mut().and_then(|live| live.pulls.get_mut(&number)) else { return };
            // A refusal stands where the run would, in Fleet's words, until a run of its kind is started.
            pull.facts.refused.retain(|(was, asked, _)| !(*was == kind && *asked == name));
            match answer {
                Ok(run) => land_run(&mut pull.facts.runs, run),
                Err(why) => pull.facts.refused.push((kind, name, why)),
            }
            this.remap(number, window, cx);
        });
    }

    /// A pull request that changed since he looked gets What changed once for its head, unless one is kept.
    pub(super) fn live_summary(&mut self, number: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(page) = self.pages.get(&number) else { return };
        if !page.changed() || page.seen.as_ref().is_some_and(|seen| seen.gone) || page.summary != model::Summary::None {
            return;
        }
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let (Some(head), Some(seen)) = (pull.facts.head.clone(), pull.facts.seen.clone()) else { return };
        let asked = pull.facts.runs.iter().any(|run| run.kind == ReviewRunKind::Explanation && run.name == WHAT_CHANGED && run.head == head && run.since.as_deref() == Some(seen.as_str()));
        let refused = pull.facts.refused.iter().any(|(kind, name, _)| *kind == ReviewRunKind::Explanation && name == WHAT_CHANGED);
        if asked || refused {
            return;
        }
        let again = pull.facts.kept_elsewhere(ReviewRunKind::Explanation, WHAT_CHANGED);
        self.start_run(number, ReviewRunKind::Explanation, WHAT_CHANGED.into(), Some(seen), again, window, cx);
    }

    /// Start: one run for each tool he ticked, over the change since he looked, or else the whole pull request.
    pub(super) fn live_critique(&mut self, number: u64, asked: &model::ReviewAsk, window: &mut Window, cx: &mut Context<Self>) {
        let since = self.pages.get(&number).filter(|page| page.changed()).and_then(|_| self.live.as_ref()?.pulls.get(&number)).filter(|pull| !pull.facts.gone).and_then(|pull| pull.facts.seen.clone());
        for tool in &asked.tools {
            self.start_run(number, ReviewRunKind::Tool, tool.clone(), since.clone(), true, window, cx);
        }
    }

    /// Find problems: the change sets, then the findings over them.
    pub(super) fn find_problems(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_live() {
            self.mock_find_problems(window, cx);
            return;
        }
        let Some(number) = self.open else { return };
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let (finds, facts) = (pull.finds.clone(), &pull.facts);
        // A run kept only at an older head is answered as it stands unless asked again.
        let overview_again = matches!(finds.flags, Find::Failed(_)) || facts.kept_elsewhere(ReviewRunKind::Overview, "");
        let sets_again = matches!(finds.sets, Find::Failed(_)) || facts.kept_elsewhere(ReviewRunKind::ChangeSets, "");
        if finds.sets == Find::Running || finds.flags == Find::Running {
            return;
        }
        self.mark_reviewing(number);
        if finds.sets == Find::Done {
            self.start_run(number, ReviewRunKind::Overview, "overview".into(), None, overview_again, window, cx);
        } else {
            if let Some(live) = self.live.as_mut() {
                live.chained.insert(number);
            }
            self.start_run(number, ReviewRunKind::ChangeSets, "change_sets".into(), None, sets_again, window, cx);
        }
        cx.notify();
    }

    /// Summarize, Explain or Simplify on Fleet: an Ask run named by its question, at the head. One
    /// written or being written stands; one that failed or was refused is asked again.
    pub(super) fn live_rewrite(&mut self, number: u64, rewrite: model::Rewrite, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let question = rewrite.question();
        let answer = map::answer_of(&pull.facts, question);
        if matches!(answer, Some(model::Answer::Writing | model::Answer::Written(_))) {
            return;
        }
        let again = matches!(answer, Some(model::Answer::Failed(_))) || pull.facts.kept_elsewhere(ReviewRunKind::Ask, question);
        self.rewritten.insert((number, rewrite), model::Answer::Writing);
        self.start_run(number, ReviewRunKind::Ask, question.into(), None, again, window, cx);
    }

    /// Check practices on one change set.
    pub(super) fn check_practices(&mut self, set: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let again = pull.finds.practices.get(&set).is_some_and(|find| matches!(find, Find::Failed(_))) || pull.facts.kept_elsewhere(ReviewRunKind::Practices, &set);
        self.start_run(number, ReviewRunKind::Practices, set, None, again, window, cx);
    }

    pub(super) fn live_finds(&self, number: u64) -> Option<&map::Finds> {
        self.live.as_ref()?.pulls.get(&number).map(|pull| &pull.finds)
    }

    // ---- What he does on a page ---------------------------------------------------------------------

    /// Viewed, kept at the head he read; cleared, forgotten. A refusal puts the box back.
    pub(super) fn live_viewed(&mut self, number: u64, path: String, viewed: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let manifest = encoded(&pull.row.manifest);
        let (route, body) = if viewed {
            let Some(head) = pull.facts.head.clone() else { return };
            let blob = pull.facts.patches.iter().find(|file| file.path == path).and_then(|file| file.blob.clone());
            (format!("/pull_request_viewed/{manifest}"), ipc::encode(&ipc::MarkFileViewed { number, path: path.clone(), commit: head, blob }).ok())
        } else {
            (format!("/pull_request_viewed/{manifest}/unmark"), ipc::encode(&ipc::UnmarkFileViewed { number, path: path.clone() }).ok())
        };
        self.call(format!("viewed:{number}:{path}"), Verb::Post, route, body, "IgnoredAny", window, cx, move |this, answer: Result<IgnoredAny, String>, _, cx| {
            if answer.is_err() {
                if let Some(file) = this.pages.get_mut(&number).and_then(|page| page.files.iter_mut().find(|file| file.path == path)) {
                    file.viewed = if viewed { model::Viewed::No } else { model::Viewed::Yes };
                }
                cx.notify();
            }
        });
    }

    /// He left the pull request: the head he leaves it at is what the next visit's change is since.
    pub(super) fn live_seen(&mut self, number: u64, cx: &mut Context<Self>) {
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let Some(head) = pull.facts.head.clone() else { return };
        if pull.facts.seen.as_deref() == Some(head.as_str()) {
            return;
        }
        let body = ipc::encode(&ipc::MarkPullSeen { number, head }).ok();
        let route = format!("/pull_request_seen/{}", encoded(&pull.row.manifest));
        self.call_quiet(format!("seen:{number}"), Verb::Post, route, body, "PullSeen", cx, move |this, answer: Result<ipc::PullSeen, String>, _| {
            if let (Ok(seen), Some(pull)) = (answer, this.live.as_mut().and_then(|live| live.pulls.get_mut(&number))) {
                pull.facts.seen = Some(seen.head);
            }
        });
    }

    /// Mark as seen in the inbox: at the row's own head, so a row never opened is marked too.
    pub(super) fn live_mark_seen(&mut self, number: u64, cx: &mut Context<Self>) {
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let body = ipc::encode(&ipc::MarkPullSeen { number, head: pull.row.head.clone() }).ok();
        let route = format!("/pull_request_seen/{}", encoded(&pull.row.manifest));
        self.set_act_said(None);
        self.call_quiet(format!("row-seen:{number}"), Verb::Post, route, body, "PullSeen", cx, move |this, answer: Result<ipc::PullSeen, String>, cx| {
            if let Err(why) = answer {
                this.row_refused(number, format!("Mark as seen on #{number} was refused. {why}"), |row| row.seen = false, cx);
            }
        });
    }

    /// Ignore, or Stop ignoring, in the inbox.
    pub(super) fn live_ignore(&mut self, number: u64, ignored: bool, cx: &mut Context<Self>) {
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let body = ipc::encode(&ipc::IgnorePull { number }).ok();
        let manifest = encoded(&pull.row.manifest);
        self.set_act_said(None);
        let key = format!("ignore:{number}");
        if ignored {
            self.call_quiet(key, Verb::Post, format!("/pull_request_ignored/{manifest}"), body, "PullIgnored", cx, move |this, answer: Result<ipc::PullIgnored, String>, cx| {
                if let Err(why) = answer {
                    this.row_refused(number, format!("Ignore on #{number} was refused. {why}"), |row| row.ignored = false, cx);
                }
            });
        } else {
            self.call_quiet(key, Verb::Post, format!("/pull_request_ignored/{manifest}/unignore"), body, "IgnorePull", cx, move |this, answer: Result<ipc::IgnorePull, String>, cx| {
                if let Err(why) = answer {
                    this.row_refused(number, format!("Stop ignoring on #{number} was refused. {why}"), |row| row.ignored = true, cx);
                }
            });
        }
    }

    /// An act on a row Fleet refused: the row goes back as it was, and the inbox says why.
    fn row_refused(&mut self, number: u64, said: String, put_back: impl FnOnce(&mut model::InboxRow), cx: &mut Context<Self>) {
        if let Some(row) = self.inbox.rows.iter_mut().find(|row| row.number == number) {
            put_back(row);
        }
        self.set_act_said(Some(said));
        cx.notify();
    }

    fn set_act_said(&mut self, said: Option<String>) {
        if let Some(live) = self.live.as_mut() {
            live.act_said = said;
        }
    }

    /// Why Fleet refused the last act on a row of the inbox.
    pub(super) fn live_act_said(&self) -> Option<String> {
        self.live.as_ref()?.act_said.clone()
    }

    /// Send: the verdict, the summary and every draft as one review at the commit he read; `bare`
    /// sends the verdict alone. A refusal changes nothing: the drafts stay and the forge's words show
    /// beside Submit.
    pub(super) fn live_submit(&mut self, verdict: Verdict, bare: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        if self.live.as_ref().is_some_and(|live| live.writing) {
            return;
        }
        let kind = match verdict {
            Verdict::Approved => ipc::PullRequestWriteKind::Approve,
            Verdict::ChangesRequested => ipc::PullRequestWriteKind::RequestChanges,
            Verdict::Commented | Verdict::NotReviewed => ipc::PullRequestWriteKind::Comment,
        };
        let comments: Vec<ipc::PullReviewComment> = self
            .drafts
            .iter()
            .filter(|draft| draft.number == number && !bare)
            .map(|draft| {
                let body = self.boxes.get(&draft.id).map(|boxed| boxed.state.read(cx).value().to_string()).unwrap_or_else(|| draft.body.clone());
                ipc::PullReviewComment::Line { path: draft.path.clone(), line: draft.line, side: ipc::DiffSide::Right, body, start_line: None }
            })
            .collect();
        let summary = self.summary.as_ref().filter(|_| !bare).map(|boxed| boxed.state.read(cx).value().trim().to_string()).filter(|said| !said.is_empty());
        let body = ipc::encode(&ipc::WritePullRequest { pull_request: number, kind, body: summary, comments, commit: pull.facts.head.clone(), session_id: None }).ok();
        let route = format!("/pull_request_writes/{}", encoded(&pull.row.manifest));
        if let Some(live) = self.live.as_mut() {
            live.writing = true;
            live.write_said.remove(&number);
        }
        cx.notify();
        self.call(format!("write:{number}"), Verb::Post, route, body, "PullRequestPage", window, cx, move |this, answer: Result<IgnoredAny, String>, window, cx| {
            if let Some(live) = this.live.as_mut() {
                live.writing = false;
            }
            match answer {
                Ok(_) => {
                    this.sent_review(number, verdict, bare, window, cx);
                    this.live_read(number, window, cx);
                }
                Err(why) => {
                    if let Some(live) = this.live.as_mut() {
                        live.write_said.insert(number, why);
                    }
                }
            }
            cx.notify();
        });
    }

    /// Send to job: his unsent notes go to the Job that holds it as one message, then Fleet records them sent.
    pub(super) fn live_send_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(number) = self.open else { return };
        let Some(pull) = self.live.as_ref().and_then(|live| live.pulls.get(&number)) else { return };
        let Some(holder) = pull.row.holder.clone() else { return };
        let unsent: Vec<&PullRequestNote> = pull.notes.iter().filter(|note| note.sent_to.is_none() && note.status == ipc::NoteStatus::Open).collect();
        if unsent.is_empty() {
            return;
        }
        let ids: Vec<String> = unsent.iter().map(|note| note.id.clone()).collect();
        let text = message_of(number, &unsent);
        let manifest = pull.row.manifest.clone();
        let body = ipc::encode(&ipc::SendSessionMessageBody {
            message: ipc::SendSessionMessage { session_id: ipc::SessionId::carried(holder.id.clone()), text, attachments: Vec::new(), mentions: Vec::new(), sketches: Vec::new() },
            aim: None,
        })
        .ok();
        self.call(format!("notes:{number}"), Verb::Post, "/sessions/message".into(), body, "IgnoredAny", window, cx, move |this, answer: Result<IgnoredAny, String>, window, cx| {
            if answer.is_err() {
                return;
            }
            this.sent_notes.insert(number);
            cx.notify();
            let body = ipc::encode(&ipc::SendPullRequestNotes { note_ids: ids, session_id: ipc::SessionId::carried(holder.id), title: holder.title }).ok();
            let route = format!("/pull_request_notes/{}/{number}/sent", encoded(&manifest));
            this.call(format!("notes-sent:{number}"), Verb::Post, route, body, "PullRequestNotes", window, cx, move |this, answer: Result<PullRequestNotes, String>, _, _| {
                if let (Ok(read), Some(pull)) = (answer, this.live.as_mut().and_then(|live| live.pulls.get_mut(&number))) {
                    pull.notes = read.notes;
                }
            });
        });
    }

    /// Why Review changes was refused on this pull request, and whether it is being sent.
    pub(super) fn live_write(&self, number: u64) -> (Option<&str>, bool) {
        self.live.as_ref().map_or((None, false), |live| (live.write_said.get(&number).map(String::as_str), live.writing))
    }

    /// The banner over the inbox's rows where they are the last read, kept.
    pub(super) fn live_inbox_banner(&self) -> Option<String> {
        self.live.as_ref()?.inbox_banner.clone()
    }

    /// What the inbox says in place of its rows while there are none: reading, or why it could not be read.
    pub(super) fn live_inbox_said(&self) -> Option<String> {
        let live = self.live.as_ref()?;
        if let Some(said) = &live.inbox_said {
            return Some(said.clone());
        }
        (self.inbox.rows.is_empty() && !self.refreshed).then(|| "Reading your pull requests".to_string())
    }

    // ---- Layouts -----------------------------------------------------------------------------------

    fn live_read_layouts(&mut self, cx: &mut Context<Self>) {
        if let Some(live) = self.live.as_mut() {
            live.layouts_read = true;
        }
        self.call_quiet("layouts".into(), Verb::Get, "/review_layouts".into(), None, "ReviewLayouts", cx, |this, answer: Result<ipc::ReviewLayouts, String>, cx| {
            if let Ok(layouts) = answer {
                this.land_layouts(layouts, true, cx);
            }
        });
    }

    /// Fleet's layouts, its Default as Bridge's own. One with a tab this screen does not draw is left out.
    fn land_layouts(&mut self, layouts: ipc::ReviewLayouts, arrange: bool, cx: &mut Context<Self>) {
        let mut all: Vec<(String, Arrangement)> = vec![(ipc::DEFAULT_REVIEW_LAYOUT.to_string(), super::default_arrangement())];
        for layout in layouts.layouts.iter().filter(|layout| !layout.shipped) {
            let kinds: Vec<&str> = super::KINDS.iter().map(|(kind, _)| *kind).collect();
            let Ok(value) = ipc::decode::<serde_json::Value>("ReviewLayout", layout.arrangement.as_bytes()) else { continue };
            if let Some(arrangement) = crate::job_view::pane_tree::revive_with(&value, &kinds, &[]) {
                all.push((layout.name.clone(), arrangement));
            }
        }
        let picked = all.iter().position(|(name, _)| *name == layouts.picked).unwrap_or(0);
        let was = self.layouts.name().to_string();
        self.layouts.set(all, picked);
        if arrange || was != self.layouts.name() {
            self.arrangement = self.layouts.arrangement();
            self.reshow(cx);
        }
        cx.notify();
    }

    fn layouts_answered(&mut self, answer: Result<ipc::ReviewLayouts, String>, cx: &mut Context<Self>) {
        if let Ok(layouts) = answer {
            self.land_layouts(layouts, false, cx);
        }
    }

    pub(super) fn live_pick_layout(&mut self, name: String, cx: &mut Context<Self>) {
        let body = ipc::encode(&ipc::NamedReviewLayout { name }).ok();
        self.call_quiet("layout-pick".into(), Verb::Post, "/review_layouts/pick".into(), body, "ReviewLayouts", cx, |this, answer, cx| this.layouts_answered(answer, cx));
    }

    pub(super) fn live_delete_layout(&mut self, name: String, cx: &mut Context<Self>) {
        let body = ipc::encode(&ipc::NamedReviewLayout { name }).ok();
        self.call_quiet("layout-delete".into(), Verb::Post, "/review_layouts/delete".into(), body, "ReviewLayouts", cx, |this, answer, cx| this.layouts_answered(answer, cx));
    }

    /// Save as: kept under the name, then picked. Rename is the same save under the new name, then the old one deleted.
    pub(super) fn live_save_layout(&mut self, name: String, arrangement: Arrangement, rename_from: Option<String>, cx: &mut Context<Self>) {
        let Ok(arrangement) = ipc::encode(&arrangement) else { return };
        let body = ipc::encode(&ipc::SaveReviewLayout { name: name.clone(), arrangement }).ok();
        self.call_quiet("layout-save".into(), Verb::Post, "/review_layouts".into(), body, "ReviewLayouts", cx, move |this, answer: Result<ipc::ReviewLayouts, String>, cx| {
            if answer.is_err() {
                this.layouts_answered(answer, cx);
                return;
            }
            if let Some(old) = rename_from {
                this.live_delete_layout(old, cx);
            }
            this.live_pick_layout(name, cx);
        });
    }
}

/// What the Job reads: each note numbered, with the file and line the forge names it by and the id to
/// resolve it with. The same words as the Job's PR panel sends (`messageOf`), which keeps its own private.
fn message_of(number: u64, notes: &[&PullRequestNote]) -> String {
    let mut lines = vec![
        format!("Notes on #{number} from the owner, kept on his Mac and not posted to the pull request. When you have dealt with a note, call resolve_pull_request_note with its id."),
        String::new(),
    ];
    for (at, note) in notes.iter().enumerate() {
        let removed = if note.side == ipc::NoteSide::Left { " on the removed line" } else { "" };
        lines.push(format!("{}. {}:{}{removed} [{}]: {}", at + 1, note.path, note.line, note.id, note.body));
    }
    lines.join("\n")
}

/// The manifest of a pull request held, or empty.
fn page_manifest(pulls: &HashMap<u64, Pull>, number: u64) -> String {
    pulls.get(&number).map(|pull| pull.row.manifest.clone()).unwrap_or_default()
}

/// A run lands over the one it moves on (same kind, name, head and range), or joins the list.
fn land_run(runs: &mut Vec<ReviewRun>, run: ReviewRun) {
    match runs.iter_mut().find(|held| held.kind == run.kind && held.name == run.name && held.head == run.head && held.since == run.since) {
        Some(held) => *held = run,
        None => runs.push(run),
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    #[gpui::test]
    fn a_pull_request_asked_of_him_opens_before_the_inbox_lists_it(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let fleet = cx.update(|cx| cx.new(FleetConnection::idle));
        let (reviews, cx) = cx.add_window_view(|window, cx| ReviewsScreen::live(&fleet, window, cx));
        let asked = ipc::ReviewAsked {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            repository: "harbor/ledger-api".into(),
            number: 2231,
            title: "Say which stage of an invoice failed".into(),
            author: "mira".into(),
            address: "https://example.com/harbor/ledger-api/pull/2231".into(),
        };
        cx.update(|window, cx| reviews.update(cx, |reviews, cx| reviews.open_asked(&asked, window, cx)));
        reviews.read_with(cx, |reviews, _| {
            let row = reviews.live.as_ref().and_then(|live| live.pulls.get(&2231)).map(|pull| (pull.row.manifest.clone(), pull.row.title.clone()));
            assert_eq!(row, Some(("ledger-api".to_string(), "Say which stage of an invoice failed".to_string())), "its page is read in the repository the request names");
            assert_eq!(reviews.showing(), Some(2231), "and it opens once read");
        });
    }

    #[gpui::test]
    fn a_page_opened_once_read_offers_its_repositorys_review_tools(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
        });
        let fleet = cx.update(|cx| cx.new(FleetConnection::idle));
        let (reviews, cx) = cx.add_window_view(|window, cx| ReviewsScreen::live(&fleet, window, cx));
        let asked = ipc::ReviewAsked {
            manifest_id: ipc::ManifestId::carried("ledger-api"),
            repository: "harbor/ledger-api".into(),
            number: 2231,
            title: "Say which stage of an invoice failed".into(),
            author: "mira".into(),
            address: "https://example.com/harbor/ledger-api/pull/2231".into(),
        };
        cx.update(|window, cx| reviews.update(cx, |reviews, cx| reviews.open_asked(&asked, window, cx)));
        // The read lands: its page and the repository's tools are held, and the page he waited for opens.
        cx.update(|window, cx| {
            reviews.update(cx, |reviews, cx| {
                let page = crate::reviews::mock::page(2231).expect("a mock page");
                reviews.pages.insert(2231, page);
                if let Some(live) = reviews.live.as_mut() {
                    live.tools.insert("ledger-api".into(), crate::reviews::mock::review_tools());
                }
                reviews.show(2231, window, cx);
            })
        });
        reviews.read_with(cx, |reviews, _| {
            assert!(!reviews.review_tools.is_empty(), "Review this pull request offers the tools Fleet read for its repository");
        });
    }
}
