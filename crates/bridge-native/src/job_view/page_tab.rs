//! A Job's page in its tab, under its card: the kept revision Fleet serves sealed for a host that draws the kit
//! (`get_artifact_page` with `host_kit`), drawn in the web view. **The page's own inline scripts run, and the kit's script
//! runs apart from them** in a world of Bridge's own (`web::KIT_WORLD`): it draws the row actions and the votes, and posts
//! each press and vote he makes here. The Job view makes the Fleet call (`press_page_action`, `cast_page_vote`) and the answer goes back
//! to the kit, which turns the row green or shows Fleet's refusal on it. The kit's look is Fleet's, never restyled here.
//! The messages are `crate::web::kit`'s; the seam is `docs/practices/protocol/a-pages-sealed-serving.md`.

use std::time::Duration;

use gpui::{div, prelude::*, px, Context, Entity, EventEmitter, Role, SharedString, Subscription, Task, Window};
use ipc::{HostToPage, PageToHost, WalkTarget};

use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::web::{finder, kit, reader, web_pane, Mode, Source, Status, WebPage, WebPageEvent, KIT_WORLD};

/// How long a walk's aim at the page waits for its target, as a web page's walk waits.
const PATIENCE: Duration = Duration::from_secs(5);
const RETRY: Duration = Duration::from_millis(150);

/// What the page tells the Job view.
pub enum PageTabEvent {
    /// The kit posted a press or a vote, for the Job view to carry to Fleet.
    Posted(PageToHost),
    /// The page's headings were read, for the reader's contents.
    Read,
}

enum Held {
    /// Fleet has not answered with the page yet: its shapes draw.
    Loading,
    /// Fleet refused the page, in its own words.
    Refused(SharedString),
    Ready(Entity<WebPage>),
}

pub struct PageTab {
    job: SharedString,
    page: String,
    revision: u32,
    /// The revision drawn: the card's own, or an earlier one the reader chose.
    shown: u32,
    held: Held,
    /// The page's headings, once it has loaded.
    heads: Vec<reader::Head>,
    /// The document's title, which its head says: a leading `h1` of the page that says it again is hidden.
    title: String,
    /// The page as it reached the web view, its sketches drawn in: what Share saves.
    html: Option<String>,
    _reading: Option<Task<()>>,
    /// The page's posts and loads, heard while it is held.
    _hearing: Option<Subscription>,
    /// A walk's aim, found and pressed in the page.
    _aim: Option<Task<()>>,
    /// The answers on their way into the page.
    _told: Vec<Task<()>>,
    /// The feeds a live graph on the page reads, watched while it is held, so a new row draws it again.
    _feeding: Option<Subscription>,
    /// What the kit was told, in order: a headless window has no page to hear it.
    #[cfg(test)]
    pub(super) answers: Vec<HostToPage>,
}

impl EventEmitter<PageTabEvent> for PageTab {}

impl PageTab {
    pub fn new(job: SharedString, page: String, revision: u32, title: String) -> Self {
        PageTab { job, page, revision, shown: revision, held: Held::Loading, heads: Vec::new(), title, html: None, _reading: None, _hearing: None, _aim: None, _told: Vec::new(), _feeding: None, #[cfg(test)] answers: Vec::new() }
    }

    /// The revision the page draws.
    pub fn shown(&self) -> u32 {
        self.shown
    }

    /// The page asked for another revision: it draws its shapes until Fleet answers.
    pub fn show_revision(&mut self, revision: u32, cx: &mut Context<Self>) {
        self.shown = revision;
        self.held = Held::Loading;
        self.html = None;
        self.heads.clear();
        self._hearing = None;
        self._feeding = None;
        cx.notify();
    }

    /// The page as it reached the web view, once Fleet has served it.
    pub fn html(&self) -> Option<&str> {
        self.html.as_deref()
    }

    /// The keys go to the page, where it is up, so what he types reaches it.
    pub fn take_keys(&self, cx: &gpui::App) {
        if let Held::Ready(page) = &self.held {
            page.read(cx).take_keys();
        }
    }

    /// The keys, if the page holds them, go back to Bridge.
    pub fn release_keys(&self, cx: &gpui::App) {
        if let Held::Ready(page) = &self.held {
            page.read(cx).release_keys();
        }
    }

    /// The headings the page holds, once it has loaded.
    pub fn heads(&self) -> &[reader::Head] {
        &self.heads
    }

    /// A script run in the page's finder world: the reader's asks. The channel answers `None` where the page is not up.
    pub fn ask(&self, script: &str, cx: &gpui::App) -> async_channel::Receiver<Option<String>> {
        match &self.held {
            Held::Ready(page) => page.read(cx).ask(script),
            _ => {
                let (tx, rx) = async_channel::bounded(1);
                let _ = tx.try_send(None);
                rx
            }
        }
    }

    /// The headings are read from the page once it has loaded.
    fn read_heads(&mut self, cx: &mut Context<Self>) {
        let ask = self.ask(&reader::words(&self.title), cx);
        self._reading = Some(cx.spawn(async move |this, cx| {
            let answer = ask.recv().await.ok().flatten();
            this.update(cx, |this, cx| {
                this.heads = reader::heads_of(answer.as_deref());
                cx.emit(PageTabEvent::Read);
                cx.notify();
            })
            .ok();
        }));
    }

    pub fn job(&self) -> &SharedString {
        &self.job
    }

    pub fn page(&self) -> &str {
        &self.page
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    /// Fleet answered with the sealed page, or refused it.
    pub fn arrived(&mut self, answer: Result<String, String>, cx: &mut Context<Self>) {
        self.held = match answer {
            Ok(html) => {
                self.html = Some(html.clone());
                let page = cx.new(|cx| WebPage::new(None, Source::Sealed(html), cx));
                // The annotation layer reaches only an offered page: without this a note covers the whole page and no part of it.
                let document = crate::page_note::Document { artifact: self.page.clone(), revision: self.shown, title: self.title.clone() };
                crate::page_note::offer_document(&page, self.job.clone(), document, cx);
                let hearing = cx.subscribe(&page, |this, page, event: &WebPageEvent, cx| match event {
                    WebPageEvent::Posted(text) => {
                        if let Some(posted) = kit::read(text) {
                            cx.emit(PageTabEvent::Posted(posted));
                        }
                    }
                    // A page drawn as a picture (under a layer, or with no window of its own) is pictured once it has loaded.
                    WebPageEvent::Loaded | WebPageEvent::Shown => {
                        if matches!(event, WebPageEvent::Loaded) && *page.read(cx).status() == Status::Ready {
                            this.read_heads(cx);
                        }
                        let pictured = *page.read(cx).status() == Status::Ready && page.read(cx).mode() == Mode::Picture;
                        if pictured {
                            page.update(cx, |page, cx| page.take_picture(cx));
                        }
                        cx.notify();
                    }
                    WebPageEvent::Pictured => cx.notify(),
                });
                self._hearing = Some(hearing);
                self._feeding = self.html.as_deref().is_some_and(super::page_graphs::holds_live).then(|| crate::graph::feeds::book(&self.job, cx)).flatten().map(|book| cx.observe(&book, |this, _, cx| this.redraw_graphs(cx)));
                Held::Ready(page)
            }
            Err(why) => Held::Refused(why.into()),
        };
        cx.notify();
    }

    /// The feeds took new rows: each live graph on the page is drawn again and put in place, as an answer is.
    fn redraw_graphs(&mut self, cx: &mut Context<Self>) {
        let (Some(html), Held::Ready(page)) = (self.html.clone(), &self.held) else { return };
        let now = super::page_graphs::redrawn(&html, &self.job, cx);
        if now == html {
            return;
        }
        let told = page.read(cx).tell(&super::page_graphs::live_script(&now));
        let page = page.clone();
        self.html = Some(now);
        self._told.retain(|task| !task.is_ready());
        self._told.push(cx.spawn(async move |_, cx| {
            told.recv().await.ok();
            page.update(cx, |page, cx| {
                if page.mode() != Mode::Live {
                    page.take_picture(cx);
                }
            });
        }));
    }

    /// Fleet's answer to a post, handed to the kit, which draws it on the row or the vote.
    pub fn answer(&mut self, reply: &HostToPage, cx: &mut Context<Self>) {
        #[cfg(test)]
        self.answers.push(reply.clone());
        let Held::Ready(page) = &self.held else { return };
        let told = page.read(cx).tell(&kit::script(reply));
        let page = page.clone();
        self._told.retain(|task| !task.is_ready());
        self._told.push(cx.spawn(async move |_, cx| {
            told.recv().await.ok();
            // Live, the page draws itself; as a picture, it is pictured again with the answer on it.
            page.update(cx, |page, cx| {
                if page.mode() != Mode::Live {
                    page.take_picture(cx);
                }
            });
        }));
    }

    /// A walk's step aimed at the page: the element of `target` is found, scrolled to the middle and ringed, and pressed
    /// where `press`, as a click the kit hears. The channel answers whether it was found within the patience.
    pub fn aim(&mut self, target: WalkTarget, press: bool, cx: &mut Context<Self>) -> async_channel::Receiver<bool> {
        let (tx, rx) = async_channel::bounded(1);
        let Held::Ready(page) = &self.held else {
            let _ = tx.try_send(false);
            return rx;
        };
        let page = page.clone();
        self._aim = Some(cx.spawn(async move |_, cx| {
            let mut waited = Duration::ZERO;
            let place = loop {
                // In the kit's world, so a press reaches the kit as his (`armadaHostPress`).
                let (ask, viewport) = page.read_with(cx, |page, _| (page.evaluate_in(KIT_WORLD, &finder::look(&target)), page.viewport()));
                let answer = ask.recv().await.ok().flatten();
                if let Some(place) = finder::place_of(answer.as_deref(), viewport) {
                    break Some(place);
                }
                if waited >= PATIENCE {
                    break None;
                }
                cx.background_executor().timer(RETRY).await;
                waited += RETRY;
            };
            let found = place.is_some();
            page.update(cx, |page, cx| page.set_ring(place, cx));
            if found && press {
                let pressed = page.read_with(cx, |page, _| page.evaluate_in(KIT_WORLD, &finder::press()));
                pressed.recv().await.ok();
                // A press moves what is on the page, so the ring would stand where the element was.
                page.update(cx, |page, cx| page.set_ring(None, cx));
            }
            page.update(cx, |page, cx| {
                if page.mode() != Mode::Live {
                    page.take_picture(cx);
                }
            });
            let _ = tx.try_send(found);
        }));
        rx
    }

    /// A walk's step that selects the words of the element it found, as a person drags over them. The channel answers
    /// once the page has run it.
    pub fn select(&mut self, cx: &mut Context<Self>) -> async_channel::Receiver<bool> {
        let (tx, rx) = async_channel::bounded(1);
        // Where `aim` found it.
        let asked = match &self.held {
            Held::Ready(page) => page.read(cx).evaluate_in(KIT_WORLD, &finder::select()),
            _ => self.ask("", cx),
        };
        self._aim = Some(cx.spawn(async move |_, _| {
            let said = asked.recv().await.ok().flatten();
            let _ = tx.try_send(said.as_deref() == Some("true"));
        }));
        rx
    }

    /// The shapes of a page that has not arrived, as a review body's.
    fn skeleton(&self, cx: &Context<Self>) -> gpui::AnyElement {
        use gpui::AnimationExt as _;
        let look = cx.look();
        let bar = |width: f32| div().h(look.length(token::SPACE_3)).w(gpui::relative(width)).rounded(look.length(token::RADIUS_SM)).bg(look.color(token::FG_DEFAULT).opacity(0.07));
        labelled(
            div().id("page-loading").w_full().p(look.length(token::SPACE_4)).child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_3))
                    .child(bar(0.4))
                    .child(bar(1.0))
                    .child(bar(0.85))
                    .child(bar(0.6))
                    .with_animation(
                        "page-loading-pulse",
                        gpui::Animation::new(look.duration(token::DURATION_PULSE)).repeat().with_easing(gpui::pulsating_between(0.5, 1.)),
                        |shapes, at| shapes.opacity(at),
                    ),
            ),
            Role::Status,
            "Loading",
        )
        .into_any_element()
    }
}

impl Render for PageTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let body = match &self.held {
            Held::Loading => self.skeleton(cx),
            Held::Refused(why) => labelled(
                div().id("page-refused").p(look.length(token::SPACE_4)).text_size(look.length(token::TEXT_SM)).text_color(look.color(token::FG_MUTED)).child(why.clone()),
                Role::Alert,
                why.clone(),
            )
            .into_any_element(),
            Held::Ready(page) => {
                // Over a picture of the page a walk's ring is gpui's; over the live page it is the page's own view.
                let ring = page.read(cx).ring_in_picture();
                let gap = look.length(token::FOCUS_RING_WIDTH) * 2.;
                div()
                    .id("page-view")
                    .relative()
                    .size_full()
                    .child(web_pane(page).size_full())
                    .children(ring.map(|place| {
                        labelled(
                            div()
                                .id("page-ring")
                                .absolute()
                                .left(px(place.frame.x as f32) - gap)
                                .top(px(place.frame.y as f32) - gap)
                                .w(px(place.frame.width as f32) + gap * 2.)
                                .h(px(place.frame.height as f32) + gap * 2.)
                                .rounded(px(place.radius as f32).max(look.length(token::RADIUS_SM)) + gap)
                                .border(look.length(token::FOCUS_RING_WIDTH))
                                .border_color(look.color(token::ACCENT)),
                            Role::Image,
                            "The target of this step",
                        )
                    }))
                    .into_any_element()
            }
        };
        labelled(div().id("page-tab").size_full().min_h(px(0.)).child(body), Role::Region, "Page")
    }
}

/// What the mock Fleet kept on one page: each press, the newest cast of each vote, and his Edits.
#[derive(Clone, Default)]
pub struct Kept {
    pub presses: Vec<ipc::PagePress>,
    pub votes: Vec<ipc::VoteCast>,
    /// The pages he saved with Edit, each the revision after the one before.
    pub his: Vec<String>,
}

impl Kept {
    /// A press or a cast the mock Fleet answered, kept as Fleet keeps it: a recast replaces the earlier one.
    fn keep(&mut self, reply: &HostToPage) {
        match reply {
            HostToPage::Pressed { press } => self.presses.push(press.clone()),
            HostToPage::Voted { cast } => {
                self.votes.retain(|one| one.vote != cast.vote);
                self.votes.push(cast.clone());
            }
            HostToPage::Refused { .. } => {}
        }
    }
}

/// The pages a Job view has open, newest last, for a walk to aim into (`page_in_front`).
#[derive(Default)]
struct Opened(Vec<gpui::WeakEntity<PageTab>>);

impl gpui::Global for Opened {}

/// The page a walk's step aims into: the newest still open.
pub(crate) fn page_in_front(cx: &gpui::App) -> Option<Entity<PageTab>> {
    cx.try_global::<Opened>()?.0.iter().rev().find_map(gpui::WeakEntity::upgrade)
}

/// What the panel keeps a live page under: the Job and the page it names.
fn panel_key(job: &str, page: &str) -> String {
    format!("{job}\u{1f}{page}")
}

impl JobView {
    /// Each document tab of Job `id` holds its page, read from Fleet once as the tab opens; a tab that closed lets its
    /// page go, so opening it again reads what he did on it since.
    pub(super) fn sync_page_tabs(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.thread(id, cx).map(|held| held.items.clone()).unwrap_or_default();
        let wanted: Vec<(String, u32, String)> = self
            .row_tab_list(id)
            .into_iter()
            .filter(|tab| tab.kind == super::row_tabs::RowKind::Document)
            .filter_map(|tab| super::review::document_card(&held, &tab.key).map(|card| (tab.key.clone(), card.revision, card.title.clone())))
            .collect();
        let tabs = self.row_tabs.entry(id.clone()).or_default();
        tabs.pages.retain(|page, (tab, _)| wanted.iter().any(|(want, revision, _)| want == page && *revision == tab.read(cx).revision()));
        let new: Vec<(String, u32, String)> = wanted.into_iter().filter(|(page, _, _)| !tabs.pages.contains_key(page)).collect();
        for (page, revision, title) in new {
            let tab = cx.new(|_| PageTab::new(id.clone(), page.clone(), revision, title));
            let carrying = cx.subscribe_in(&tab, window, |this, tab, event: &PageTabEvent, window, cx| {
                match event {
                    PageTabEvent::Posted(posted) => this.carry(tab.clone(), posted.clone(), window, cx),
                    PageTabEvent::Read => cx.notify(),
                }
            });
            let mut opened = cx.try_global::<Opened>().map(|kept| kept.0.clone()).unwrap_or_default();
            opened.retain(|one| one.upgrade().is_some());
            opened.push(tab.downgrade());
            cx.set_global(Opened(opened));
            self.row_tabs.entry(id.clone()).or_default().pages.insert(page, (tab.clone(), carrying));
            self.fetch_page(tab, window, cx);
        }
        self.sync_readers(id, window, cx);
    }

    /// The pages the item chosen in the review panel reads, a document's own or those of an artifact's page parts, each hold a live
    /// page, read from Fleet once the item is picked and let go with the panel, so the panel draws the page as its tab does and
    /// never a picture of it. A page whose revision the Job drew again is read again, and a document he opens is told seen.
    pub(super) fn sync_panel_pages(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = self.review_chosen().filter(|_| self.review_open());
        let wanted: Vec<(String, u32, String)> = chosen.as_ref().map(|item| self.panel_pages_of(item, cx)).unwrap_or_default();
        let job = chosen.as_ref().map(|item| SharedString::from(item.job.clone()));
        let is_document = chosen.as_ref().is_some_and(|item| item.kind == super::review::Kind::Document);
        let Some(review) = self.review_mut() else { return };
        review.pages.retain(|key, (tab, _)| wanted.iter().any(|(page, revision, _)| job.as_ref().is_some_and(|job| *key == panel_key(job, page)) && *revision == tab.read(cx).revision()));
        let Some(job) = job else { return };
        let new: Vec<(String, u32, String)> = wanted.into_iter().filter(|(page, _, _)| !review.pages.contains_key(&panel_key(&job, page))).collect();
        for (page, revision, title) in new {
            let tab = cx.new(|_| PageTab::new(job.clone(), page.clone(), revision, title));
            let carrying = cx.subscribe_in(&tab, window, |this, tab, event: &PageTabEvent, window, cx| match event {
                PageTabEvent::Posted(posted) => this.carry(tab.clone(), posted.clone(), window, cx),
                PageTabEvent::Read => {}
            });
            let mut opened = cx.try_global::<Opened>().map(|kept| kept.0.clone()).unwrap_or_default();
            opened.retain(|one| one.upgrade().is_some());
            opened.push(tab.downgrade());
            cx.set_global(Opened(opened));
            if let Some(review) = self.review_mut() {
                review.pages.insert(panel_key(&job, &page), (tab.clone(), carrying));
            }
            self.fetch_page(tab, window, cx);
            if is_document {
                self.panel_seen(&job, &page, revision, window, cx);
            }
        }
    }

    /// He opened revision `revision` of document `page`: Fleet is told, as the reader tells it, so a page he was to read stops
    /// waiting.
    fn panel_seen(&mut self, job: &SharedString, page: &str, revision: u32, window: &mut Window, cx: &mut Context<Self>) {
        if self.artifact_of(job, page).and_then(|one| one.seen).is_some_and(|seen| seen >= revision) {
            return;
        }
        if self.fleet.is_none() {
            self.artifacts_seen_post(job, page, revision);
            return;
        }
        let body = ipc::encode(&ipc::MarkArtifactSeen { session_id: ipc::SessionId::carried(job.to_string()), artifact: page.to_string(), revision });
        let artifact = page.to_string();
        self.post(
            "/sessions/artifacts/seen".to_string(),
            body,
            window,
            cx,
            Box::new(move |_, answer, _, _| {
                if let Err(why) = answer {
                    crate::log::Note::warn("Fleet did not take that he opened a revision").field("artifact", artifact).field("revision", revision).field("why", why).emit();
                }
            }),
        );
    }

    /// The live page the review panel holds for page `page` of Job `job` at `revision`, where it has one.
    pub(super) fn panel_page(&self, job: &str, page: &str, revision: u32, cx: &gpui::App) -> Option<Entity<PageTab>> {
        self.review()?.pages.get(&panel_key(job, page)).map(|(tab, _)| tab).filter(|tab| tab.read(cx).revision() == revision).cloned()
    }

    /// The page, sealed, as Fleet serves it (`get_artifact_page`), or Fleet's refusal of it.
    pub(super) fn fetch_page(&mut self, tab: Entity<PageTab>, window: &mut Window, cx: &mut Context<Self>) {
        let (job, page, revision) = {
            let held = tab.read(cx);
            (held.job().clone(), held.page().to_string(), held.shown())
        };
        if self.fleet.is_none() {
            let kept = self.row_tabs.get(&job).and_then(|tabs| tabs.mock_kept.get(&page)).cloned().unwrap_or_default();
            let answer = crate::mock::sealed_revision(&page, revision, &kept.his, &kept.presses, &kept.votes).map(|html| super::page_graphs::drawn(&super::page_sketches::drawn(&html, &page, window, cx), &job, cx));
            tab.update(cx, |tab, cx| tab.arrived(answer, cx));
            return;
        }
        // `host_kit`: the page's own scripts run, and Bridge draws the kit apart from them.
        let path = format!("/sessions/artifacts/page?session_id={}&artifact={}&revision={revision}&host_kit=true", super::wire::encoded(&job), super::wire::encoded(&page));
        let named = page.clone();
        let job_of = job.clone();
        let done: super::wire::Done = Box::new(move |_, answer, window, cx| {
            let answer = answer.and_then(|bytes| String::from_utf8(bytes).map_err(|_| "Fleet's page is not text".to_string()));
            // The sketches and graphs the page holds are drawn before the page reaches its web view (`page_sketches.rs`, `page_graphs.rs`).
            let answer = answer.map(|html| super::page_graphs::drawn(&super::page_sketches::drawn(&html, &named, window, cx), &job_of, cx));
            tab.update(cx, |tab, cx| tab.arrived(answer, cx));
        });
        self.get(path, window, cx, done);
    }

    /// A press or a vote the kit posted, carried to Fleet, and Fleet's answer handed back to the kit.
    fn carry(&mut self, tab: Entity<PageTab>, posted: PageToHost, window: &mut Window, cx: &mut Context<Self>) {
        let job = tab.read(cx).job().clone();
        if self.fleet.is_none() {
            let answer = crate::mock::page_answer(&posted);
            let reply = kit::reply(&posted, answer.as_deref().map_err(Clone::clone));
            let page = tab.read(cx).page().to_string();
            self.row_tabs.entry(job.clone()).or_default().mock_kept.entry(page).or_default().keep(&reply);
            self.answered(&tab, &reply, cx);
            return;
        }
        let (route, body) = kit::call(&job, &posted);
        let done: super::wire::Done = Box::new(move |this, answer, _, cx| {
            let code = this.refused_code.clone();
            let reply = kit::reply(&posted, answer.as_deref().map_err(|why| (why.clone(), code)));
            this.answered(&tab, &reply, cx);
        });
        self.post(route.into(), body, window, cx, done);
    }

    /// Fleet's answer goes to the kit; a Job a press started raises a toast that opens it.
    fn answered(&mut self, tab: &Entity<PageTab>, reply: &HostToPage, cx: &mut Context<Self>) {
        tab.update(cx, |tab, cx| tab.answer(reply, cx));
        let Some(started) = kit::started(reply) else { return };
        let (job, page) = {
            let held = tab.read(cx);
            (held.job().clone(), held.page().to_string())
        };
        let from = self.thread(&job, cx).and_then(|held| super::review::document_card(&held.items, &page)).map_or_else(|| "A page".to_string(), |card| card.title.clone());
        let named = self.title_of(started.as_str(), cx).map_or_else(|| format!("Job {}", started.as_str().chars().take(8).collect::<String>()), |title| title.to_string());
        crate::toasts::job_started(started.as_str(), format!("{from} started {named}"), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn a_document_that_arrives_is_offered_to_the_notes_layer_for_its_job(cx: &mut gpui::TestAppContext) {
        let tab = cx.new(|_| PageTab::new("job-1".into(), "design".into(), 2, "How a Job lands".into()));
        tab.update(cx, |tab, cx| tab.arrived(Ok("<p>Three branches are stale now.</p>".into()), cx));
        let page = tab.read_with(cx, |tab, _| match &tab.held {
            Held::Ready(page) => page.clone(),
            _ => panic!("the page is held"),
        });
        let (job, document) = cx.update(|cx| (crate::page_note::offered_for(&page, cx), crate::page_note::document_of(&page, cx)));
        assert_eq!(job.as_deref(), Some("job-1"), "a note on the document goes to its own Job");
        assert_eq!(document, Some(crate::page_note::Document { artifact: "design".into(), revision: 2, title: "How a Job lands".into() }));
    }

    fn press(action: &str) -> PageToHost {
        PageToHost::Press { artifact: "design".into(), revision: 2, action: action.into(), words: None }
    }

    #[test]
    fn the_mock_page_opened_again_carries_what_fleet_kept_and_not_what_it_refused() {
        let mut kept = Kept::default();
        for action in ["discuss-dark-mode", "drop-checkout"] {
            let posted = press(action);
            let answer = crate::mock::page_answer(&posted);
            kept.keep(&kit::reply(&posted, answer.as_deref().map_err(Clone::clone)));
        }
        let pick = |take: &str| PageToHost::Vote { artifact: "design".into(), revision: 2, vote: "mark".into(), result: ipc::VoteResult::Pick { take: take.into(), notes: Default::default() } };
        for take in ["chip", "fade"] {
            let posted = pick(take);
            kept.keep(&kit::reply(&posted, crate::mock::page_answer(&posted).as_deref().map_err(Clone::clone)));
        }
        assert_eq!(kept.presses.iter().map(|one| one.action.as_str()).collect::<Vec<_>>(), ["discuss-dark-mode"], "a refused press keeps nothing");
        assert_eq!(kept.votes.len(), 1, "a recast replaces the earlier cast");
        let page = crate::mock::sealed_page("design", 2, &kept.presses, &kept.votes).expect("the design page is served");
        assert!(page.contains(r#""presses":[{"revision":2,"action":"discuss-dark-mode""#), "the press rides in the record");
        assert!(page.contains(r#""take":"fade""#), "the newest cast rides in the record");
        assert!(!page.contains("window.armadaHost = hear"), "Bridge draws the kit itself, so Fleet seals none in");
        assert!(crate::mock::sealed_page("notes", 1, &[], &[]).is_err(), "a page the mock never kept is refused");
    }

    #[test]
    fn the_mock_serves_the_first_revision_of_the_design_page_and_the_marks_page_with_a_sketch_in_each_take() {
        let first = crate::mock::sealed_page("design", 1, &[], &[]).expect("revision 1 is kept");
        assert!(first.contains("Three branches are stale now.") && first.contains(r#""revision":1"#));
        assert!(crate::mock::sealed_page("design", 3, &[], &[]).is_err());
        let marks = crate::mock::sealed_page("marks", 1, &[], &[]).expect("the marks page is served");
        assert_eq!(marks.matches("<armada-sketch").count(), 3, "its own drawing and one in each take");
        assert!(marks.contains(r#"<armada-take id="chip""#) && marks.contains(r#"{"artifact":"marks","revision":1,"presses":[],"votes":[]}"#));
    }

    #[test]
    fn the_mock_refuses_as_fleet_does_with_its_code() {
        assert_eq!(crate::mock::page_answer(&press("drop-checkout")).unwrap_err().1.as_deref(), Some("fleet.page_ask_not_open"));
        assert_eq!(crate::mock::page_answer(&press("merge-it")).unwrap_err().1.as_deref(), Some("fleet.page_action_unknown"));
        let wrong = PageToHost::Vote { artifact: "design".into(), revision: 2, vote: "row-parts".into(), result: ipc::VoteResult::Pick { take: "full".into(), notes: Default::default() } };
        assert_eq!(crate::mock::page_answer(&wrong).unwrap_err().1.as_deref(), Some("fleet.page_vote_form"));
    }
}
