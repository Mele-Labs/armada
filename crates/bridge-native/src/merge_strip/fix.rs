//! The red band's acts that ask something first (`MainHead.tsx`): **Send back** hands the red to a Job as a
//! message, and **Take over** replaces a claim another machine holds. Each opens a dialog over the window; the
//! strip emits what was chosen and the wiring does it. A Job in `Recent` is one the person could send work to,
//! the culprit's own first.

use gpui::{
    anchored, deferred, div, point, prelude::*, px, Anchor, AnyElement, Context, Entity, FocusHandle, FontWeight, Pixels, Role, SharedString,
    Window,
};
use gpui_component::input::{Textarea, TextareaState};
use ipc::{FixingElsewhere, MainMerge, MainRedJob, MainView};

use super::{Asked, MergeStrip, Recent};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{card, glyph, mono, small, Btn, Weight};

/// A claim held this long reads as stale by its mark and age alone.
const STALE_AFTER_MINUTES: i64 = 60;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Which {
    Back,
    TakeOver,
}

/// The dialog that is open, with what the red said when it opened.
pub(super) struct FixDialog {
    which: Which,
    root: String,
    failed: MainRedJob,
    merge: Option<MainMerge>,
    elsewhere: Option<FixingElsewhere>,
    /// The Jobs to send it to, the Job that merged the pull request first.
    sessions: Vec<Recent>,
    picked: Option<SharedString>,
    /// The words that go with the work: the brief, there to be edited once a Job is picked.
    request: Entity<TextareaState>,
    focus: FocusHandle,
}

/// The brief a new Job starts from: bare facts, there to be edited (`briefOf`).
pub(super) fn brief_of(failed: &MainRedJob, merge: Option<&MainMerge>) -> String {
    let mut lines = vec![format!("{} fails on main.", failed.check)];
    if let Some(test) = &failed.test {
        lines.push(format!("Test: {test}"));
    }
    if let Some(merge) = merge {
        let branch = merge.branch.as_ref().map(|branch| format!(" ({branch})")).unwrap_or_default();
        lines.push(format!("Merged in #{}{branch}.", merge.number));
    }
    lines.join("\n")
}

impl MergeStrip {
    /// The first Job to offer is the culprit's, and it is already chosen before the person looks.
    pub(super) fn open_fix(&mut self, line: usize, which: Which, window: &mut Window, cx: &mut Context<Self>) {
        let Some(one) = self.data.lines.get(line) else { return };
        let Some(MainView::Red { failed, merge, elsewhere, .. }) = &one.view.main else { return };
        let (root, failed, merge, elsewhere) = (one.view.root.clone(), failed.clone(), merge.clone(), elsewhere.clone());
        let culprit = merge.as_ref().and_then(|merge| merge.session.as_ref()).map(|session| session.id.clone());
        let mut sessions: Vec<Recent> = self.data.recent.clone();
        sessions.sort_by_key(|one| Some(one.id.to_string()) != culprit);
        let picked = culprit.and_then(|id| sessions.first().filter(|first| first.id == id).map(|first| first.id.clone()));
        let brief = brief_of(&failed, merge.as_ref());
        let request = cx.new(|cx| {
            let mut state = TextareaState::new(window, cx).auto_grow(5, 10);
            state.set_value(brief, window, cx);
            state
        });
        self.keep_keys(window, cx);
        self.card = None;
        self.pinned = false;
        self.by_keys = None;
        self.over_card = false;
        self._ticking = None;
        let focus = cx.focus_handle();
        let handle = focus.clone();
        self.fix = Some(FixDialog { which, root, failed, merge, elsewhere, sessions, picked, request, focus });
        window.defer(cx, move |window, cx| window.focus(&handle, cx));
        cx.notify();
    }

    fn fix_cancelled(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.fix = None;
        self.give_keys_back(window, cx);
        cx.notify();
    }

    /// Whether the dialog may be confirmed: a Job is picked and has words.
    fn fix_ready(&self, cx: &gpui::App) -> bool {
        self.fix.as_ref().is_some_and(|fix| match fix.which {
            Which::TakeOver => true,
            Which::Back => fix.picked.is_some() && !fix.request.read(cx).value().trim().is_empty(),
        })
    }

    fn fix_confirmed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fix_ready(cx) {
            return;
        }
        let Some(fix) = self.fix.take() else { return };
        let words = fix.request.read(cx).value().to_string();
        self.fixing = Some(fix.root.clone());
        match (fix.which, fix.picked) {
            (Which::Back, Some(session)) => cx.emit(Asked::SendBack { root: fix.root, session, message: words }),
            _ => cx.emit(Asked::TakeOver { root: fix.root, request: brief_of(&fix.failed, fix.merge.as_ref()) }),
        }
        self.give_keys_back(window, cx);
        cx.notify();
    }

    /// The dialog over the window, with a scrim that a press on shuts it through.
    pub(super) fn fix_layer(&self, look: &Look, viewport: gpui::Size<Pixels>, cx: &Context<Self>) -> Option<AnyElement> {
        let fix = self.fix.as_ref()?;
        let (title, confirm): (SharedString, &'static str) = match fix.which {
            Which::Back => ("Send the work back".into(), "Send back"),
            Which::TakeOver => (format!("Take over from {}", fix.elsewhere.as_ref().map(|held| held.machine.as_str()).unwrap_or_default()).into(), "Take over"),
        };
        let body = match fix.which {
            Which::Back => self.back_body(fix, look, cx),
            Which::TakeOver => fix.elsewhere.as_ref().map(|held| held_by(held, look, None)).unwrap_or_else(|| div().into_any_element()),
        };
        let ready = self.fix_ready(cx);
        let width = look.length(token::W_DIALOG) * if fix.which == Which::Back { 1.25 } else { 1. };
        let scrim = div()
            .id("fix-scrim")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p(look.length(token::SPACE_6))
            .bg(look.color(token::SCRIM))
            .occlude()
            .on_click(cx.listener(|this, _, window, cx| this.fix_cancelled(window, cx)))
            .child(labelled(
                card(look)
                    .id("fix-dialog")
                    .track_focus(&fix.focus)
                    .tab_group()
                    .capture_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                        let key = &event.keystroke;
                        match key.key.as_str() {
                            "escape" => {
                                this.fix_cancelled(window, cx);
                                cx.stop_propagation();
                            }
                            "enter" if key.modifiers.platform || this.fix.as_ref().is_some_and(|fix| fix.focus.is_focused(window)) => {
                                this.fix_confirmed(window, cx);
                                cx.stop_propagation();
                            }
                            _ => {
                                if crate::keymap::traverse(event, window, cx) {
                                    cx.stop_propagation();
                                }
                            }
                        }
                    }))
                    // A press inside the dialog is not a press on the scrim.
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .w(width)
                    .max_w_full()
                    .gap(look.length(token::SPACE_4))
                    .p(look.length(token::PAD_CARD))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .text_size(look.length(token::TEXT_BASE))
                            .line_height(look.length(token::LEADING_BASE))
                            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                            .child(title.clone()),
                    )
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(look.length(token::SPACE_2))
                            .child(labelled(
                                Btn::new("fix-cancel", "Cancel", Weight::Secondary).draw(look, cx).on_click(cx.listener(|this, _, window, cx| this.fix_cancelled(window, cx))),
                                Role::Button,
                                "Cancel",
                            ))
                            .child(labelled(
                                Btn::new("fix-confirm", confirm, Weight::Primary)
                                    .disabled(!ready)
                                    .draw(look, cx)
                                    .on_click(cx.listener(|this, _, window, cx| this.fix_confirmed(window, cx))),
                                Role::Button,
                                confirm,
                            )),
                    ),
                Role::Dialog,
                title,
            ));
        Some(
            deferred(anchored().position(point(px(0.), px(0.))).anchor(Anchor::TopLeft).child(div().w(viewport.width).h(viewport.height).child(scrim)))
                .with_priority(2)
                .into_any_element(),
        )
    }

    /// What goes with the work, the Jobs to send it to and, once one is picked, the words.
    fn back_body(&self, fix: &FixDialog, look: &Look, cx: &Context<Self>) -> AnyElement {
        let chip = |from: &str, name: String| {
            labelled(
                div()
                    .id(SharedString::from(format!("attached-{from}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_2))
                    .h(look.length(token::H_BADGE) + look.length(token::SPACE_1))
                    .rounded(look.length(token::RADIUS_SM))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .font_family(mono(look))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().text_color(look.color(token::FG_MUTED)).child(from.to_string()))
                    .child(name.clone()),
                Role::ListItem,
                format!("{from} {name}"),
            )
        };
        let mut chips = vec![chip(if fix.failed.unmapped { "CI job" } else { "Check" }, fix.failed.check.clone())];
        if let Some(test) = &fix.failed.test {
            chips.push(chip("Test", test.clone()));
        }
        chips.push(chip("Log", format!("{}.log", fix.failed.check)));
        if let Some(merge) = &fix.merge {
            let branch = merge.branch.as_ref().map(|branch| format!(" {branch}")).unwrap_or_default();
            chips.push(chip("Pull request", format!("#{}{branch}", merge.number)));
        }
        let culprit = fix.merge.as_ref().and_then(|merge| merge.session.as_ref()).map(|session| session.id.clone());
        let jobs = (!fix.sessions.is_empty()).then(|| {
            let rows = fix.sessions.iter().map(|one| {
                let id = one.id.clone();
                let on = fix.picked.as_ref() == Some(&one.id);
                let said = fix.merge.as_ref().map(|merge| format!("Merged #{}", merge.number));
                let mark = (Some(one.id.to_string()) == culprit).then(|| {
                    labelled(
                        div().id(SharedString::from(format!("culprit-{}", one.id))).flex_none().child(glyph("git-merge", look.length(token::SPACE_3), look.color(token::FG_MUTED))),
                        Role::Image,
                        said.unwrap_or_default(),
                    )
                });
                labelled(
                    div()
                        .id(SharedString::from(format!("job-{}", one.id)))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .cursor_pointer()
                        .focusable()
                        .tab_stop(true)
                        .focus_visible(|style| style.bg(look.color(token::BG_HOVER)))
                        .hover(|style| style.bg(look.color(token::BG_HOVER)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(fix) = this.fix.as_mut() {
                                fix.picked = Some(id.clone());
                            }
                            cx.notify();
                        }))
                        .child(glyph(if on { "circle-dot" } else { "circle" }, look.length(token::SPACE_4), look.color(if on { token::ACCENT } else { token::FG_MUTED })))
                        .child(div().child(one.title.clone()))
                        .children(one.branch.clone().map(|branch| div().font_family(mono(look)).text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(branch)))
                        .children(mark),
                    Role::RadioButton,
                    one.title.clone(),
                )
            });
            labelled(div().id("fix-jobs").flex().flex_col().gap(look.length(token::SPACE_1)).children(rows), Role::RadioGroup, "Jobs")
        });
        let words = fix.picked.is_some().then(|| {
            div()
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .child(small(look).text_color(look.color(token::FG_MUTED)).child("Message"))
                .child(labelled(div().id("fix-message").child(Textarea::new(&fix.request)), Role::TextInput, "Message"))
        });
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .child(labelled(div().id("fix-attached").flex().flex_wrap().gap(look.length(token::SPACE_2)).children(chips), Role::List, "Attached"))
            .children(jobs)
            .children(words)
            .into_any_element()
    }
}

/// Another machine's claim: who holds it, and for how long. A claim long held reads as stale by its mark and age alone.
pub(super) fn held_by(held: &FixingElsewhere, look: &Look, act: Option<AnyElement>) -> AnyElement {
    let since = chrono::DateTime::parse_from_rfc3339(held.since.as_str()).ok().map(|at| (chrono::Utc::now() - at.with_timezone(&chrono::Utc)).num_minutes().max(0));
    let stale = since.is_some_and(|minutes| minutes >= STALE_AFTER_MINUTES);
    let age = since.map(|minutes| match minutes {
        0 => "now".to_string(),
        1..=59 => format!("{minutes}m"),
        60..=1439 => format!("{}h", minutes / 60),
        _ => format!("{}d", minutes / 1440),
    });
    let ink = look.color(if stale { token::STATUS_AWAITING_REVIEW } else { token::FG_MUTED });
    div()
        .flex()
        .items_center()
        .gap(look.length(token::SPACE_3))
        .child(labelled(
            div()
                .id("held-holder")
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(glyph("square-terminal", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(div().font_family(mono(look)).child(held.holder.id.clone()))
                .child(glyph("monitor", look.length(token::SPACE_3), look.color(token::FG_MUTED)))
                .child(held.machine.clone()),
            Role::Group,
            format!("Job {} on {}", held.holder.id, held.machine),
        ))
        .children(age.map(|age| {
            labelled(
                div()
                    .id("held-age")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_1))
                    .font_family(mono(look))
                    .text_color(ink)
                    .children(stale.then(|| glyph("clock", look.length(token::SPACE_3), ink)))
                    .child(age),
                Role::Group,
                if stale { "Claimed, stale" } else { "Claimed" },
            )
        }))
        .children(act)
        .into_any_element()
}
