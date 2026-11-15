//! The Fleet panel's Add a branch fold (#3547): branches origin holds that main lacks and no Job here holds,
//! newest first. A search narrows them, since the list runs long; a plus picks one into the next build, where
//! it stands above the search with an x, and the Waiting section's build of the picks follows.

use gpui::{div, prelude::*, px, AnyElement, Context, Entity, Role, SharedString, Subscription, Window};
use gpui_component::input::{Input, InputEvent, InputState};

use super::face::{built_said, Build};
use super::parts::{icon_button, mono, tip};
use super::switcher::{row, state_glyph, Row};
use super::{Asked, TitleBar};
use crate::labels::labelled;
use crate::look::{token, Look};

/// The rows drawn before the list scrolls.
const ROWS_SHOWN: f32 = 6.;

/// The search over the On origin row: made when the Fleet panel first opens, and kept with what was typed.
#[derive(Default)]
pub(super) struct OriginSearch {
    input: Option<(Entity<InputState>, Subscription)>,
    query: String,
}

impl OriginSearch {
    /// The search's keys, once it is made.
    pub(super) fn focus(&self, cx: &gpui::App) -> Option<gpui::FocusHandle> {
        use gpui::Focusable as _;
        self.input.as_ref().map(|(input, _)| input.read(cx).focus_handle(cx))
    }
}

/// The branches whose name holds every word typed, in Fleet's order.
pub(super) fn matching<'a>(origin: &'a [ipc::OriginBranch], query: &str) -> Vec<&'a ipc::OriginBranch> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    origin
        .iter()
        .filter(|one| {
            let name = one.branch.to_lowercase();
            words.iter().all(|word| name.contains(word.as_str()))
        })
        .collect()
}

/// How long ago a branch was last committed to, in its largest unit (`4m`, `3h`, `2d`). Text that is no
/// instant has no age.
pub(super) fn age_of(at: &ipc::Instant, now: chrono::DateTime<chrono::Utc>) -> Option<String> {
    let at = chrono::DateTime::parse_from_rfc3339(at.as_str()).ok()?;
    let minutes = (now - at.with_timezone(&chrono::Utc)).num_minutes().max(0);
    Some(match minutes {
        0..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    })
}

impl TitleBar {
    /// The search box, made once a window is at hand.
    pub(super) fn make_origin_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.origin.input.is_some() {
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search branches"));
        let typed = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                this.origin.query = input.read(cx).value().to_string();
                cx.notify();
            }
        });
        self.origin.input = Some((input, typed));
    }

    /// Add a branch, opened: the branches he added to the next build, each with its x, then the search and the
    /// branches on origin. Nothing while there are neither.
    pub(super) fn origin_section(&self, held: &Build, added: &[&ipc::PickedJob], look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let origin = &held.report.origin;
        if origin.is_empty() && added.is_empty() {
            return None;
        }
        let off = held.restart().pending || held.report.swapping || held.report.reverting;
        let added_rows: Vec<AnyElement> = added
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let name: SharedString = one.title.clone().unwrap_or_else(|| one.branch.clone()).into();
                let branch = one.branch.clone();
                let leave: SharedString = format!("Leave {} out of the build", one.branch).into();
                let x = icon_button(("unpick", at), "x", look.color(token::FG_MUTED), look).size(look.length(token::H_CONTROL_SM)).tooltip(tip("Leave out of the next build"));
                let x = x.on_click(cx.listener(move |this, _, _, cx| this.unpick_pressed(branch.clone(), cx)));
                row(
                    look,
                    "added",
                    at,
                    Row {
                        mark: state_glyph(format!("added-mark-{at}").into(), "plus", look.color(token::STATUS_COMPLETED_SUCCESS), "Added to the next build".into()),
                        hover: one.title.is_some().then(|| one.branch.clone().into()),
                        name,
                        about: None,
                        word: None,
                        end: Some(labelled(x, Role::Button, leave).into_any_element()),
                    },
                )
            })
            .collect();
        let now = chrono::Utc::now();
        let shown = matching(origin, &self.origin.query);
        let row_height = look.length(token::H_CONTROL_LG);
        let rows: Vec<AnyElement> = shown
            .iter()
            .enumerate()
            .map(|(at, one)| {
                let branch = one.branch.clone();
                let name: SharedString = one.title.clone().unwrap_or_else(|| one.branch.clone()).into();
                let hover: SharedString = match &one.title {
                    Some(title) => format!("{title}, {}", one.branch).into(),
                    None => one.branch.clone().into(),
                };
                let age = age_of(&one.committed_at, now);
                let when: SharedString = format!("Last commit {}", built_said(&one.committed_at)).into();
                let pick: SharedString = format!("Add {} to the next build", one.branch).into();
                let plus = icon_button(("origin-pick", at), "plus", look.color(if off { token::FG_SUBTLE } else { token::FG_MUTED }), look)
                    .size(look.length(token::H_CONTROL_SM))
                    .tooltip(tip("Add to the next build"));
                let plus = if off { plus.opacity(0.5) } else { plus.on_click(cx.listener(move |this, _, _, cx| this.origin_pick_pressed(branch.clone(), cx))) };
                div()
                    .id(("origin-row", at))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .h(row_height)
                    .pl(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_LG))
                    .hover(|style| style.bg(look.color(token::BG_HOVER)))
                    .child(div().id(("origin-name", at)).flex_1().min_w_0().truncate().when(one.title.is_none(), |name| name.font_family(mono(look))).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).tooltip(tip(hover)).child(name.clone()))
                    .children(age.map(|age| {
                        labelled(
                            div()
                                .id(("origin-age", at))
                                .flex_none()
                                .font_family(mono(look))
                                .text_size(look.length(token::TEXT_XS))
                                .line_height(look.length(token::LEADING_XS))
                                .text_color(look.color(token::FG_SUBTLE))
                                .tooltip(tip(when.clone()))
                                .child(age),
                            Role::Image,
                            when,
                        )
                    }))
                    .child(labelled(plus, Role::Button, pick))
                    .into_any_element()
            })
            .collect();
        let search = self.origin.input.as_ref().map(|(input, _)| {
            labelled(
                div().id("origin-search").child(
                    Input::new(input)
                        .h(look.length(token::H_CONTROL))
                        .rounded(look.length(token::RADIUS_LG))
                        .text_size(look.length(token::TEXT_XS))
                        .prefix(super::parts::glyph("search", look.length(token::SPACE_3), look.color(token::FG_SUBTLE))),
                ),
                Role::SearchInput,
                "Search branches on origin",
            )
        });
        let none = rows.is_empty().then(|| {
            div().px(look.length(token::SPACE_2)).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child("No branch matches")
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .children((!added_rows.is_empty()).then(|| {
                    labelled(
                        div().id("added").flex().flex_col().overflow_hidden().rounded(look.length(token::RADIUS_LG)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).bg(look.color(token::BG_SUNKEN)).children(added_rows),
                        Role::List,
                        "Added to the next build",
                    )
                }))
                .children((!origin.is_empty()).then_some(search).flatten())
                .children((!origin.is_empty()).then_some(none).flatten())
                .child(labelled(
                    div().id("origin-list").flex().flex_col().max_h(row_height * ROWS_SHOWN + px(1.)).overflow_y_scroll().children(rows),
                    Role::List,
                    "On origin",
                ))
                .into_any_element(),
        )
    }

    /// A branch on origin goes into the next build: it moves to the picked chips in the same frame, and
    /// Fleet's snapshot starts over the picks.
    fn origin_pick_pressed(&mut self, branch: String, cx: &mut Context<Self>) {
        let taken = branch.clone();
        self.answer(
            move |face| {
                if let Some(held) = face.build.as_mut() {
                    held.report.origin.retain(|one| one.branch != taken);
                    if !held.report.picked.iter().any(|one| one.branch == taken) {
                        held.report.picked.push(ipc::PickedJob { branch: taken, title: None });
                    }
                }
            },
            cx,
        );
        cx.emit(Asked::PickOrigin(branch));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch(name: &str, at: &str) -> ipc::OriginBranch {
        ipc::OriginBranch { branch: name.into(), committed_at: ipc::Instant::carried(at), title: None }
    }

    #[test]
    fn a_search_keeps_the_branches_holding_every_word_in_fleets_order() {
        let origin = [branch("native/refresh-set", "2026-11-13T10:00:00Z"), branch("native/on-origin", "2026-11-13T09:00:00Z"), branch("fleet/origin-fetch", "2026-11-12T09:00:00Z")];
        let names = |query: &str| matching(&origin, query).iter().map(|one| one.branch.as_str()).collect::<Vec<_>>();
        assert_eq!(names(""), ["native/refresh-set", "native/on-origin", "fleet/origin-fetch"]);
        assert_eq!(names("ORIGIN"), ["native/on-origin", "fleet/origin-fetch"]);
        assert_eq!(names("native origin"), ["native/on-origin"]);
        assert!(names("nothing").is_empty());
    }

    #[test]
    fn an_age_is_said_in_its_largest_unit_and_text_that_is_no_instant_has_none() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-13T12:00:00Z").unwrap().with_timezone(&chrono::Utc);
        let age = |at: &str| age_of(&ipc::Instant::carried(at), now);
        assert_eq!(age("2026-11-13T11:56:00Z").as_deref(), Some("4m"));
        assert_eq!(age("2026-11-13T09:00:00Z").as_deref(), Some("3h"));
        assert_eq!(age("2026-11-11T11:00:00Z").as_deref(), Some("2d"));
        assert_eq!(age("2026-11-13T12:30:00Z").as_deref(), Some("0m"));
        assert_eq!(age("yesterday"), None);
    }
}
