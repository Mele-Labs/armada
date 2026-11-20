//! Walks: each walk the Job showed, with where it stands. One that waits on him reads "waits on you"; one he approved
//! reads "approved" with its time. A press opens the walk as a press on its Needs you row does. The Job's own words about
//! the walks come from its thread: a `window` row for each walk shown, and his `Approved:` message for each he approved.

use chrono::Local;
use gpui::{div, prelude::*, AnyElement, Context, Role, SharedString, Window};
use ipc::{Instant, SessionRow, SessionVoice};

use super::rows::{approved_address, clock, glyph, tip, walk_of, Size};
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
        let SessionRow::Window { url, title, ended_at, .. } = row else { continue };
        let stand = match ended_at {
            None => Stand::Waits,
            Some(at) if approved.iter().any(|address| same_walk(address, url)) => Stand::Approved(at.clone()),
            Some(at) => Stand::Closed(at.clone()),
        };
        let walked = Walked { url: url.clone(), title: title.clone(), stand };
        match out.iter_mut().find(|one| same_walk(&one.url, url)) {
            Some(one) => *one = walked,
            None => out.push(walked),
        }
    }
    out
}

impl JobView {
    pub(super) fn pane_walks(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let walks = walks_of(&self.thread_rows(id, cx));
        if walks.is_empty() {
            return None;
        }
        let rows: Vec<AnyElement> = walks.iter().map(|walk| self.walk_row(walk, look, cx)).collect();
        Some(labelled(div().id(SharedString::from(format!("pane-walks:{id}"))).flex().flex_col().children(rows), Role::List, "Walks").into_any_element())
    }

    fn walk_row(&self, walk: &Walked, look: &Look, cx: &Context<Self>) -> AnyElement {
        let now = Local::now();
        let (mark, hue, tooltip) = match walk.stand {
            Stand::Waits => ("eye", token::STATUS_AWAITING_REVIEW, "Waits on you"),
            Stand::Approved(_) => ("circle-check", token::STEP_ADVANCED, "Approved"),
            Stand::Closed(_) => ("eye", token::FG_SUBTLE, "Closed without your Approve"),
        };
        let said = match &walk.stand {
            Stand::Waits => "waits on you".to_string(),
            Stand::Approved(at) => format!("approved {}", clock(at.as_str(), now).unwrap_or_else(|| at.as_str().to_string())),
            Stand::Closed(at) => format!("closed {}", clock(at.as_str(), now).unwrap_or_else(|| at.as_str().to_string())),
        };
        let key = walk.url.clone();
        let open = key.clone();
        labelled(
            div()
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
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tab_index(0)
                .focus_visible(|style| style.border_1().border_color(look.color(token::ACCENT)))
                .child(div().id(SharedString::from(format!("walk-mark:{key}"))).flex_none().tooltip(tip(SharedString::from(tooltip))).child(glyph(look, mark, Size::Small, look.color(hue))))
                .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(walk.title.clone())))
                .child(div().flex_none().text_color(look.color(if walk.stand == Stand::Waits { token::STATUS_AWAITING_REVIEW } else { token::FG_SUBTLE })).child(SharedString::from(said.clone())))
                .on_click(cx.listener(move |this, _, window: &mut Window, cx| this.press_walk(&open, window, cx))),
            Role::Button,
            format!("{}, {said}", walk.title),
        )
        .into_any_element()
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
}
