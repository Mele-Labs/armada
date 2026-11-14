//! A Job's page in its tab, under its card: the kept revision Fleet serves sealed (`get_artifact_page`), drawn in the web
//! view. **The kit's script on it is the page's only script**: it draws the row actions and the votes, and posts each
//! press and vote here. The Job view makes the Fleet call (`press_page_action`, `cast_page_vote`) and the answer goes back
//! to the kit, which turns the row green or shows Fleet's refusal on it. The kit's look is Fleet's, never restyled here.
//! The messages are `crate::web::kit`'s; the seam is `docs/practices/protocol/a-pages-sealed-serving.md`.

use std::time::Duration;

use gpui::{div, prelude::*, px, Context, Entity, EventEmitter, Role, SharedString, Subscription, Task, Window};
use ipc::{HostToPage, PageToHost, WalkTarget};

use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};
use crate::web::{finder, kit, web_pane, Mode, Source, Status, WebPage, WebPageEvent};

/// How long a walk's aim at the page waits for its target, as a web page's walk waits.
const PATIENCE: Duration = Duration::from_secs(5);
const RETRY: Duration = Duration::from_millis(150);

/// What the page tells the Job view.
pub enum PageTabEvent {
    /// The kit posted a press or a vote, for the Job view to carry to Fleet.
    Posted(PageToHost),
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
    held: Held,
    /// The page's posts and loads, heard while it is held.
    _hearing: Option<Subscription>,
    /// A walk's aim, found and pressed in the page.
    _aim: Option<Task<()>>,
    /// The answers on their way into the page.
    _told: Vec<Task<()>>,
}

impl EventEmitter<PageTabEvent> for PageTab {}

impl PageTab {
    pub fn new(job: SharedString, page: String, revision: u32) -> Self {
        PageTab { job, page, revision, held: Held::Loading, _hearing: None, _aim: None, _told: Vec::new() }
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
                let page = cx.new(|cx| WebPage::new(None, Source::Sealed(html), cx));
                let hearing = cx.subscribe(&page, |_, page, event: &WebPageEvent, cx| match event {
                    WebPageEvent::Posted(text) => {
                        if let Some(posted) = kit::read(text) {
                            cx.emit(PageTabEvent::Posted(posted));
                        }
                    }
                    // A page drawn as a picture (under a layer, or with no window of its own) is pictured once it has loaded.
                    WebPageEvent::Loaded | WebPageEvent::Shown => {
                        let pictured = *page.read(cx).status() == Status::Ready && page.read(cx).mode() == Mode::Picture;
                        if pictured {
                            page.update(cx, |page, cx| page.take_picture(cx));
                        }
                        cx.notify();
                    }
                    WebPageEvent::Pictured => cx.notify(),
                });
                self._hearing = Some(hearing);
                Held::Ready(page)
            }
            Err(why) => Held::Refused(why.into()),
        };
        cx.notify();
    }

    /// Fleet's answer to a post, handed to the kit, which draws it on the row or the vote.
    pub fn answer(&mut self, reply: &HostToPage, cx: &mut Context<Self>) {
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
                let (ask, viewport) = page.read_with(cx, |page, _| (page.ask(&finder::look(&target)), page.viewport()));
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
                let pressed = page.read_with(cx, |page, _| page.ask(&finder::press()));
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

/// What the mock Fleet kept on one page: each press, and the newest cast of each vote.
#[derive(Clone, Default)]
pub struct Kept {
    pub presses: Vec<ipc::PagePress>,
    pub votes: Vec<ipc::VoteCast>,
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

impl JobView {
    /// Each document tab of Job `id` holds its page, read from Fleet once as the tab opens; a tab that closed lets its
    /// page go, so opening it again reads what he did on it since.
    pub(super) fn sync_page_tabs(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let held = self.thread(id, cx).map(|held| held.items.clone()).unwrap_or_default();
        let wanted: Vec<(String, u32)> = self
            .row_tab_list(id)
            .into_iter()
            .filter(|tab| tab.kind == super::row_tabs::RowKind::Document)
            .filter_map(|tab| super::review::document_card(&held, &tab.key).map(|card| (tab.key.clone(), card.revision)))
            .collect();
        let tabs = self.row_tabs.entry(id.clone()).or_default();
        tabs.pages.retain(|page, (tab, _)| wanted.iter().any(|(want, revision)| want == page && *revision == tab.read(cx).revision()));
        let new: Vec<(String, u32)> = wanted.into_iter().filter(|(page, _)| !tabs.pages.contains_key(page)).collect();
        for (page, revision) in new {
            let tab = cx.new(|_| PageTab::new(id.clone(), page.clone(), revision));
            let carrying = cx.subscribe_in(&tab, window, |this, tab, event: &PageTabEvent, window, cx| {
                let PageTabEvent::Posted(posted) = event;
                this.carry(tab.clone(), posted.clone(), window, cx);
            });
            let mut opened = cx.try_global::<Opened>().map(|kept| kept.0.clone()).unwrap_or_default();
            opened.retain(|one| one.upgrade().is_some());
            opened.push(tab.downgrade());
            cx.set_global(Opened(opened));
            self.row_tabs.entry(id.clone()).or_default().pages.insert(page, (tab.clone(), carrying));
            self.fetch_page(tab, window, cx);
        }
    }

    /// The page, sealed, as Fleet serves it (`get_artifact_page`), or Fleet's refusal of it.
    fn fetch_page(&mut self, tab: Entity<PageTab>, window: &mut Window, cx: &mut Context<Self>) {
        let (job, page, revision) = {
            let held = tab.read(cx);
            (held.job().clone(), held.page().to_string(), held.revision())
        };
        if self.fleet.is_none() {
            let kept = self.row_tabs.get(&job).and_then(|tabs| tabs.mock_kept.get(&page)).cloned().unwrap_or_default();
            let answer = crate::mock::sealed_page(&page, revision, &kept.presses, &kept.votes);
            tab.update(cx, |tab, cx| tab.arrived(answer, cx));
            return;
        }
        let path = format!("/sessions/artifacts/page?session_id={}&artifact={}&revision={revision}", super::wire::encoded(&job), super::wire::encoded(&page));
        let done: super::wire::Done = Box::new(move |_, answer, _, cx| {
            let answer = answer.and_then(|bytes| String::from_utf8(bytes).map_err(|_| "Fleet's page is not text".to_string()));
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
        assert!(page.contains("window.armadaHost = hear"), "the kit Fleet sealed it with");
        assert!(crate::mock::sealed_page("notes", 1, &[], &[]).is_err(), "a page the mock never kept is refused");
    }

    #[test]
    fn the_mock_refuses_as_fleet_does_with_its_code() {
        assert_eq!(crate::mock::page_answer(&press("drop-checkout")).unwrap_err().1.as_deref(), Some("fleet.page_ask_not_open"));
        assert_eq!(crate::mock::page_answer(&press("merge-it")).unwrap_err().1.as_deref(), Some("fleet.page_action_unknown"));
        let wrong = PageToHost::Vote { artifact: "design".into(), revision: 2, vote: "row-parts".into(), result: ipc::VoteResult::Pick { take: "full".into(), notes: Default::default() } };
        assert_eq!(crate::mock::page_answer(&wrong).unwrap_err().1.as_deref(), Some("fleet.page_vote_form"));
    }
}
