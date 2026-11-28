//! Walks: each walk the Job showed, with where it stands. One that waits on him reads "waits on you"; one he approved
//! reads "approved" with its time. A press opens the walk as a press on its Needs you row does. The Job's own words about
//! the walks come from its thread: a `window` row for each walk shown, and his `Approved:` message for each he approved.

use chrono::Local;
use gpui::{div, prelude::*, AnimationExt, AnyElement, Context, Role, SharedString, Window};
use ipc::{Instant, SessionRow, SessionVoice, WalkBuild, WalkBuildState};

use super::rows::{approved_address, clock, glyph, tip, walk_of, Size};
use crate::look::Token;
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look};

/// Where a walk stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Stand {
    /// Open: shown, and not approved or closed.
    Waits,
    /// He approved it, when the window ended.
    Approved(Instant),
    /// Its window ended with no approval of his in the thread.
    Closed(Instant),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Walked {
    pub(super) url: String,
    pub(super) title: String,
    pub(super) stand: Stand,
    /// Where the build of a native walk from the Job's branch stands; none for a web page.
    pub(super) build: Option<WalkBuild>,
}

/// What a walk's row draws and does: the mark with its tooltip, the words, and whether a press opens the walk.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Reads {
    pub(super) mark: &'static str,
    pub(super) hue: Token,
    pub(super) tooltip: &'static str,
    pub(super) said: String,
    pub(super) said_hue: Token,
    /// The mark turns while Fleet builds.
    pub(super) spins: bool,
    pub(super) opens: bool,
}

/// How the row reads: a walk still being built or whose build failed says so and opens nothing; any other is as it stands.
pub(super) fn reads(walk: &Walked, now: chrono::DateTime<Local>) -> Reads {
    if walk.stand == Stand::Waits {
        match walk.build.as_ref().map(|build| build.state) {
            Some(WalkBuildState::Building) => {
                return Reads { mark: "loader-circle", hue: token::STEP_RUNNING, tooltip: "Fleet is building this walk from the Job's branch", said: "Building".into(), said_hue: token::STEP_RUNNING, spins: true, opens: false };
            }
            Some(WalkBuildState::Failed) => {
                return Reads { mark: "circle-x", hue: token::STEP_FAILED, tooltip: "The build failed and the Job was told", said: "Not ready".into(), said_hue: token::FG_SUBTLE, spins: false, opens: false };
            }
            Some(WalkBuildState::Ready) | None => {}
        }
    }
    let (mark, hue, tooltip) = match walk.stand {
        Stand::Waits => ("eye", token::STATUS_AWAITING_REVIEW, "Waits on you"),
        Stand::Approved(_) => ("circle-check", token::STEP_ADVANCED, "Approved"),
        Stand::Closed(_) => ("eye", token::FG_SUBTLE, "Closed without your Approve"),
    };
    let when = |at: &Instant| clock(at.as_str(), now).unwrap_or_else(|| at.as_str().to_string());
    let said = match &walk.stand {
        Stand::Waits => "waits on you".to_string(),
        Stand::Approved(at) => format!("approved {}", when(at)),
        Stand::Closed(at) => format!("closed {}", when(at)),
    };
    let said_hue = if walk.stand == Stand::Waits { token::STATUS_AWAITING_REVIEW } else { token::FG_SUBTLE };
    Reads { mark, hue, tooltip, said, said_hue, spins: false, opens: true }
}

/// Whether two addresses are the one walk: the same address, or the same `walk` name on either.
fn same_walk(one: &str, other: &str) -> bool {
    one == other || walk_of(one).is_some_and(|name| walk_of(other) == Some(name))
}

/// The walks the thread shows, in the order they were first shown. A walk shown again keeps its place and takes the
/// newest row's title and state.
pub(super) fn walks_of(rows: &[SessionRow]) -> Vec<Walked> {
    let approved: Vec<&str> = rows
        .iter()
        .filter_map(|row| match row {
            SessionRow::Message { from: SessionVoice::You, text, .. } => approved_address(text),
            _ => None,
        })
        .collect();
    let mut out: Vec<Walked> = Vec::new();
    for row in rows {
        let SessionRow::Window { url, title, ended_at, build, .. } = row else { continue };
        let stand = match ended_at {
            None => Stand::Waits,
            Some(at) if approved.iter().any(|address| same_walk(address, url)) => Stand::Approved(at.clone()),
            Some(at) => Stand::Closed(at.clone()),
        };
        let walked = Walked { url: url.clone(), title: title.clone(), stand, build: build.clone() };
        match out.iter_mut().find(|one| same_walk(&one.url, url)) {
            Some(one) => *one = walked,
            None => out.push(walked),
        }
    }
    out
}

/// Whether Fleet is still building the walk at this address: its newest showing is open and its build is Building.
pub(super) fn is_building(rows: &[SessionRow], url: &str) -> bool {
    walks_of(rows).iter().find(|walk| same_walk(&walk.url, url)).is_some_and(|walk| walk.stand == Stand::Waits && walk.build.as_ref().is_some_and(|build| build.state == WalkBuildState::Building))
}

/// A mark that turns while Fleet builds.
pub(super) fn spinning(mark: gpui::Svg, key: &str, look: &Look) -> AnyElement {
    mark.with_animation(SharedString::from(format!("walk-building:{key}")), gpui::Animation::new(look.duration(token::DURATION_SPIN)).repeat(), |mark, at| {
        mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(at)))
    })
    .into_any_element()
}

impl JobView {
    /// Whether Fleet is still building the walk at `url`, which Job `id` showed.
    pub(super) fn walk_building(&self, id: &str, url: &str, cx: &gpui::App) -> bool {
        is_building(&self.thread_rows(id, cx), url)
    }

    pub(super) fn pane_walks(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let walks = walks_of(&self.thread_rows(id, cx));
        if walks.is_empty() {
            return None;
        }
        let rows: Vec<AnyElement> = walks.iter().map(|walk| self.walk_row(walk, look, cx)).collect();
        Some(labelled(div().id(SharedString::from(format!("pane-walks:{id}"))).flex().flex_col().children(rows), Role::List, "Walks").into_any_element())
    }

    fn walk_row(&self, walk: &Walked, look: &Look, cx: &Context<Self>) -> AnyElement {
        let Reads { mark, hue, tooltip, said, said_hue, spins, opens } = reads(walk, Local::now());
        let key = walk.url.clone();
        let open = key.clone();
        let glyph = glyph(look, mark, Size::Small, look.color(hue));
        let glyph = if spins { spinning(glyph, &key, look) } else { glyph.into_any_element() };
        let row = div()
            .id(SharedString::from(format!("walk-row:{key}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_h(look.length(token::H_ROW))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(labelled(div().id(SharedString::from(format!("walk-mark:{key}"))).flex_none().tooltip(tip(SharedString::from(tooltip))).child(glyph), Role::Image, tooltip))
            .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(walk.title.clone())))
            .child(div().flex_none().text_color(look.color(said_hue)).child(SharedString::from(said.clone())));
        // A walk still building or not ready is a row that says so; it takes no focus and no press.
        let row = if opens {
            row.cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .on_click(cx.listener(move |this, _, window: &mut Window, cx| this.press_walk(&open, window, cx)))
        } else {
            row
        };
        labelled(row, if opens { Role::Button } else { Role::Row }, format!("{}, {said}", walk.title)).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<SessionRow> {
        let row = |json: &str| ipc::decode::<SessionRow>("row", json.as_bytes()).expect("the row decodes");
        vec![
            row(r#"{"kind":"window","id":"w1","at":"2026-11-09T10:20:00Z","title":"Keys for each pane","url":"armada://walk/keys","ended_at":"2026-11-09T10:42:00Z"}"#),
            row(r#"{"kind":"message","id":"m1","at":"2026-11-09T10:42:00Z","from":{"kind":"you"},"text":"Approved: armada://walk/keys"}"#),
            row(r#"{"kind":"window","id":"w2","at":"2026-11-09T10:50:00Z","title":"Settings overlay","url":"armada://walk/overlay"}"#),
            row(r#"{"kind":"window","id":"w3","at":"2026-11-09T10:55:00Z","title":"Sidebar","url":"armada://walk/sidebar","ended_at":"2026-11-09T10:58:00Z"}"#),
        ]
    }

    /// What the page draws: the one he approved says so with its time, the open one waits on him, and a window that ended
    /// with no approval of his is not called approved.
    #[test]
    fn a_walk_reads_waiting_approved_or_closed_from_the_thread() {
        let walks = walks_of(&rows());
        assert_eq!(walks.iter().map(|walk| walk.title.as_str()).collect::<Vec<_>>(), ["Keys for each pane", "Settings overlay", "Sidebar"], "in the order shown");
        assert!(matches!(&walks[0].stand, Stand::Approved(at) if at.as_str() == "2026-11-09T10:42:00Z"));
        assert_eq!(walks[1].stand, Stand::Waits);
        assert!(matches!(walks[2].stand, Stand::Closed(_)));
    }

    /// A walk shown again is one row, with the newest state, and a Job that showed none has no tab body.
    #[test]
    fn a_walk_shown_again_is_one_row_and_a_thread_with_none_has_none() {
        let mut held = rows();
        held.push(ipc::decode::<SessionRow>("row", br#"{"kind":"window","id":"w4","at":"2026-11-09T11:00:00Z","title":"Keys for each pane","url":"armada://walk/keys"}"#).expect("decodes"));
        let walks = walks_of(&held);
        assert_eq!(walks.len(), 3, "the same walk, one row");
        assert_eq!(walks[0].stand, Stand::Waits, "shown again, it waits again");
        assert!(walks_of(&[]).is_empty());
    }

    /// The Job in the mock that is on the Walk step holds one he approved and one that waits on him; the tab lists both,
    /// each named as he would read it, and a press is a button.
    #[gpui::test]
    fn the_walks_tab_lists_each_walk_with_where_it_stands(cx: &mut gpui::TestAppContext) {
        use super::super::column_strip::tests::drawn::{fresh, opened};
        use super::super::parts_fold::drawn_like;
        let (view, cx) = opened(cx, "mock-16");
        view.update_in(cx, |view, _, cx| view.open_kind("walks", cx));
        fresh(cx);
        let rows = drawn_like(cx, gpui::Role::Button, "Settings overlay");
        assert_eq!(rows.len(), 1, "the walk is a row of the tab");
        assert!(rows[0].contains("waits on you"), "{}", rows[0]);
        let done = drawn_like(cx, gpui::Role::Button, "Keys for each pane");
        assert_eq!(done.len(), 1);
        assert!(done[0].contains("approved"), "{}", done[0]);
    }

    fn built(state: Option<&str>) -> Walked {
        let build = state.map(|state| format!(r#","build":{{"state":"{state}","commit":"abc1234","at":"2026-11-09T10:51:00Z"}}"#)).unwrap_or_default();
        let json = format!(r#"{{"kind":"window","id":"w9","at":"2026-11-09T10:50:00Z","title":"Walk bar","url":"armada://walk/bar"{build}}}"#);
        let row = ipc::decode::<SessionRow>("row", json.as_bytes()).expect("the row decodes");
        walks_of(&[row]).remove(0)
    }

    /// A walk Fleet is still building says Building and opens nothing; one whose build failed says Not ready, muted, and
    /// opens nothing; a ready one or a page with no build reads and opens as a walk always did.
    #[test]
    fn a_walk_reads_and_opens_by_where_its_build_stands() {
        let now = Local::now();
        let building = reads(&built(Some("building")), now);
        assert_eq!((building.said.as_str(), building.opens, building.spins, building.mark), ("Building", false, true, "loader-circle"));
        assert_eq!(building.tooltip, "Fleet is building this walk from the Job's branch");
        let failed = reads(&built(Some("failed")), now);
        assert_eq!((failed.said.as_str(), failed.opens, failed.spins), ("Not ready", false, false));
        assert_eq!(failed.said_hue, token::FG_SUBTLE);
        assert_eq!(failed.tooltip, "The build failed and the Job was told");
        for state in [Some("ready"), None] {
            let ready = reads(&built(state), now);
            assert_eq!((ready.said.as_str(), ready.opens, ready.spins), ("waits on you", true, false));
        }
    }

    // ---- a walk still building reaches him at once (his ruling, 28 Nov 2026) ----

    const JOB: &str = "tool-edit";
    const BUILDING: &str = "armada://walk/walkBarButtons";
    const READY: &str = "armada://walk/stripMarks";

    fn on_screen(cx: &mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &mut gpui::VisualTestContext) {
        let (view, cx) = super::super::parts_fold::rooted_job_with(cx, "walk-builds", JOB, gpui::size(gpui::px(1440.), gpui::px(900.)), |_| {}, |_| {});
        view.update(cx, |view, cx| view.sync_walk_tabs(&JOB.into(), cx));
        super::super::column_strip::tests::drawn::fresh(cx);
        (view, cx)
    }

    /// The walk in front of the focused pane, by address.
    fn in_front(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> Option<String> {
        view.read_with(cx, |view, _| {
            let arrangement = view.arrangement().into_owned();
            let tab = super::super::pane_tree::pane_by_id(&arrangement, &arrangement.focus).and_then(super::super::panes_place::front_of)?;
            (tab.kind == "walk").then(|| tab.row.clone()).flatten()
        })
    }

    fn opened_by_fleet(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext) -> usize {
        view.read_with(cx, |view, _| view.mock_posted.iter().filter(|(path, _)| path == "/sessions/window/open").count())
    }

    fn front(view: &gpui::Entity<JobView>, cx: &mut gpui::VisualTestContext, url: &str) {
        let job: SharedString = JOB.into();
        view.update_in(cx, |view, window, cx| view.row_tab_front(&job, super::super::row_tabs::RowKind::Walk, url, window, cx));
        super::super::column_strip::tests::drawn::fresh(cx);
    }

    /// The tab of a walk still building says Building, and its Open is greyed with the reason; a built walk's Open is as it was.
    #[gpui::test]
    fn the_tab_of_a_building_walk_says_building_and_greys_open(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::drawn;
        let (view, cx) = on_screen(cx);
        front(&view, cx, BUILDING);
        assert!(drawn(cx, gpui::Role::Status, "Building"), "the body says Building");
        assert!(drawn(cx, gpui::Role::Button, "Open, opens once built"), "Open is greyed and says why");
        assert!(!drawn(cx, gpui::Role::Button, "Open"));
        front(&view, cx, READY);
        assert!(drawn(cx, gpui::Role::Button, "Open") && !drawn(cx, gpui::Role::Status, "Building"), "a built walk opens");
    }

    /// A press on a walk still building, from any of the places that open a walk, brings its tab forward and asks Fleet for
    /// nothing; the same press on a built walk asks Fleet to open it.
    #[gpui::test]
    fn a_press_on_a_building_walk_brings_its_tab_forward_and_opens_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = on_screen(cx);
        assert_eq!(in_front(&view, cx), None, "the thread is in front");
        view.update_in(cx, |view, window, cx| view.press_walk(BUILDING, window, cx));
        super::super::column_strip::tests::drawn::fresh(cx);
        assert_eq!(in_front(&view, cx).as_deref(), Some(BUILDING), "its tab came forward");
        assert_eq!(opened_by_fleet(&view, cx), 0, "Fleet was not asked to open it");
        // The greyed Open in its own tab does the same: the tab stays in front, and nothing is asked.
        view.update_in(cx, |view, window, cx| view.open_walk(BUILDING, true, window, cx));
        assert_eq!(opened_by_fleet(&view, cx), 0);
        view.update_in(cx, |view, window, cx| view.press_walk(READY, window, cx));
        assert_eq!(opened_by_fleet(&view, cx), 1, "a built walk is opened by the same press");
    }

    /// Needs you holds the walk from the moment it is shown, and says Building on it until the build is Ready.
    #[gpui::test]
    fn the_needs_you_item_and_the_row_of_a_building_walk_read_building(cx: &mut gpui::TestAppContext) {
        use super::super::parts_fold::{drawn_like, redraw};
        let (view, cx) = on_screen(cx);
        view.update_in(cx, |view, window, cx| view.set_needs(true, None, window, cx));
        redraw(cx);
        let item = drawn_like(cx, gpui::Role::Button, "Open Walk the bar buttons");
        assert!(!item.is_empty() && item.iter().all(|name| name == "Open Walk the bar buttons, Building"), "the item reads Building: {item:?}");
        let built = drawn_like(cx, gpui::Role::Button, "Open Walk the strip marks");
        assert!(!built.is_empty() && built.iter().all(|name| name == "Open Walk the strip marks"), "a built walk's item is as it was: {built:?}");
        view.update_in(cx, |view, window, cx| view.set_needs(false, None, window, cx));
        view.update_in(cx, |view, _, cx| view.open_kind("spine", cx));
        redraw(cx);
        let row = drawn_like(cx, gpui::Role::Button, "Walk: Walk bar buttons");
        assert_eq!(row, ["Walk: Walk bar buttons, Building"], "the Job's row reads Building");
    }
}
