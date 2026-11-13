//! A description a Job proposed for the open pull request (`PullRequestPage.draft`), drawn in place of the
//! description on Conversation: the draft with what it changes in the current one marked, Current to read
//! the one it replaces, and Approve, Edit and Note answering its `draft:<id>` item. Armada Classic draws
//! none of it; the draft waits in the thread there.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{canvas, div, point, prelude::*, px, AnyElement, Context, Entity, FontWeight, Role, SharedString, Subscription, Window};
use gpui_component::input::{Enter, InputEvent, Textarea, TextareaState};
use ipc::{AnswerWaiting, PullRequestDraft, PullRequestPage, SessionId};

use super::super::rows::{glyph, mono, tip, Size};
use super::super::JobView;
use super::{small_button, strip_comments, PullPanel};
use crate::labels::labelled;
use crate::look::{token, Look};

/// What a run of the draft's lines does to the current description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::job_view) enum Change {
    Same,
    Added,
    Removed,
}

/// The box open under the draft: its words as fixed, or a note that sends it back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::job_view) enum Writing {
    Edit,
    Note,
}

impl Writing {
    fn word(self) -> &'static str {
        match self {
            Writing::Edit => "edit",
            Writing::Note => "note",
        }
    }
}

/// The draft's own state in the open panel.
#[derive(Default)]
pub(in crate::job_view) struct DraftPane {
    /// Current is shown in place of the draft.
    pub current: bool,
    pub writing: Option<(Writing, Entity<TextareaState>)>,
    pub sending: bool,
    pub said: Option<SharedString>,
    /// The draft Fleet took an answer for, and what it was, until the page is read without it.
    pub settled: Option<(String, u32, &'static str)>,
    /// The box has just opened: the next frame scrolls the panel so it and its buttons are in view.
    reveal: Rc<Cell<bool>>,
    _typing: Option<Subscription>,
}

/// Runs of lines `after` keeps, adds and drops against `before`, by their longest common run of lines.
/// A line changed in place reads as the old one dropped, then the new one added.
pub(in crate::job_view) fn changes(before: &str, after: &str) -> Vec<(Change, Vec<String>)> {
    let a: Vec<&str> = before.trim().lines().map(str::trim_end).collect();
    let b: Vec<&str> = after.trim().lines().map(str::trim_end).collect();
    let (n, m) = (a.len(), b.len());
    let mut common = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            common[i][j] = if a[i] == b[j] { common[i + 1][j + 1] + 1 } else { common[i + 1][j].max(common[i][j + 1]) };
        }
    }
    let mut out: Vec<(Change, Vec<String>)> = Vec::new();
    let mut push = |kind: Change, line: &str| match out.last_mut() {
        Some((last, lines)) if *last == kind => lines.push(line.to_string()),
        _ => out.push((kind, vec![line.to_string()])),
    };
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            push(Change::Same, b[j]);
            i += 1;
            j += 1;
        } else if i < n && (j == m || common[i + 1][j] >= common[i][j + 1]) {
            push(Change::Removed, a[i]);
            i += 1;
        } else {
            push(Change::Added, b[j]);
            j += 1;
        }
    }
    out
}

fn kept_key(draft: &PullRequestDraft, writing: Writing) -> String {
    format!("pull-request:draft:{}:{}", draft.draft, writing.word())
}

impl JobView {
    /// The draft's card, in the description's place at the head of Conversation.
    pub(in crate::job_view) fn pull_draft_card(&self, panel: &PullPanel, page: &PullRequestPage, draft: &PullRequestDraft, look: &Look, cx: &Context<Self>) -> AnyElement {
        let pane = &panel.draft;
        let current = strip_comments(page.body.as_deref().unwrap_or(""));
        let has_current = !current.trim().is_empty();
        let showing_current = pane.current && has_current;
        let hue = look.color(token::STATUS_AWAITING_REVIEW);
        let edge = look.length(token::BORDER_WIDTH);

        let shown = |current_one: bool| {
            let front = showing_current == current_one;
            let (word, said, on) = if current_one {
                ("Current", if has_current { "The description the pull request has now" } else { "The pull request has no description yet" }, has_current)
            } else {
                ("Draft", "The draft, with what it changes marked", true)
            };
            labelled(
                div()
                    .id(SharedString::from(format!("pull-draft-show:{word}")))
                    .px(look.length(token::SPACE_2))
                    .h(look.length(token::H_CONTROL_SM) - look.length(token::SPACE_1))
                    .flex()
                    .items_center()
                    .rounded(look.length(token::RADIUS_SM))
                    .text_size(look.length(token::TEXT_XS))
                    .font_weight(FontWeight(look.number(if front { token::WEIGHT_HEADING } else { token::WEIGHT_MEDIUM })))
                    .text_color(look.color(if front { token::FG_DEFAULT } else { token::FG_MUTED }))
                    .when(front, |tab| tab.bg(look.color(token::BG_RAISED)))
                    .when(!on, |tab| tab.opacity(0.5))
                    .tooltip(tip(said))
                    .when(on, |tab| {
                        tab.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(panel) = this.pull_panel_mut() {
                                panel.draft.current = current_one;
                            }
                            cx.notify();
                        }))
                    })
                    .child(word),
                Role::Tab,
                word,
            )
        };
        let toggle = labelled(
            div().id("pull-draft-shown").flex().items_center().gap(gpui::px(2.)).p(gpui::px(2.)).rounded(look.length(token::RADIUS_MD)).bg(look.color(token::BG_SUNKEN)).child(shown(false)).child(shown(true)),
            Role::TabList,
            "Description shown",
        );
        let revision = format!("Revision {}", draft.revision);
        let chip = labelled(
            div()
                .id("pull-draft-revision")
                .flex_none()
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .bg(hue.opacity(0.16))
                .text_color(hue)
                .font_family(mono(look))
                .text_size(look.length(token::TEXT_2XS))
                .line_height(look.length(token::LEADING_2XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .tooltip(tip(format!("{revision} of the Job's description for this pull request")))
                .child(SharedString::from(format!("r{}", draft.revision))),
            Role::Status,
            revision,
        );
        let head = div()
            .flex()
            .items_center()
            .flex_wrap()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .bg(look.color(token::BG_OVERLAY).blend(hue.opacity(0.10)))
            .border_b(edge)
            .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.3)))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .child(glyph(look, "file-pen-line", Size::Small, hue))
            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Description draft"))
            .child(chip)
            .child(div().flex_1())
            .child(toggle);

        let words = if showing_current {
            div().min_w_0().child(self.prose("pull-draft:current".to_string(), &current, look, cx)).into_any_element()
        } else {
            self.draft_runs(draft, &current, look, cx)
        };
        let body = div().min_w_0().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_3)).child(words);

        let settled = pane.settled.as_ref().filter(|(id, revision, _)| *id == draft.draft && *revision == draft.revision).map(|(_, _, word)| *word);
        let foot = match settled {
            Some(word) => labelled(
                div()
                    .id("pull-draft-settled")
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .child(glyph(look, "circle-check", Size::Small, look.color(token::STEP_ADVANCED)))
                    .child(word),
                Role::Status,
                word,
            )
            .into_any_element(),
            None => self.draft_foot(panel, draft, look, cx),
        };
        let foot = div().flex().flex_col().gap(look.length(token::SPACE_2)).px(look.length(token::SPACE_3)).pb(look.length(token::SPACE_3)).child(foot);

        labelled(
            div()
                .id("pull-draft")
                .relative()
                .min_w_0()
                .overflow_hidden()
                .border(edge)
                .border_color(look.color(token::BORDER_SUBTLE).blend(hue.opacity(0.5)))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::BG_RAISED))
                .child(div().absolute().left_0().top_0().bottom_0().w(edge * 3.).bg(hue))
                .child(head)
                .child(body)
                .child(foot),
            Role::Article,
            "Description draft",
        )
        .into_any_element()
    }

    /// The draft as runs of lines: those it keeps as written, those it adds in the advanced hue, and those it
    /// drops struck through in the failed hue.
    fn draft_runs(&self, draft: &PullRequestDraft, current: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        let text = strip_comments(&draft.text);
        if current.trim().is_empty() || current.trim() == text.trim() {
            return div().min_w_0().child(self.prose("pull-draft:text".to_string(), &text, look, cx)).into_any_element();
        }
        let edge = look.length(token::BORDER_WIDTH);
        let runs = changes(current, &text).into_iter().enumerate().map(|(at, (kind, lines))| {
            let words = lines.join("\n");
            match kind {
                // The same inset and edge as a band, unmarked, so every line starts at one left edge.
                Change::Same => div()
                    .min_w_0()
                    .pl(look.length(token::SPACE_3))
                    .pr(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1))
                    .border_l(edge * 3.)
                    .border_color(gpui::transparent_black())
                    .child(self.prose(format!("pull-draft:same:{at}"), &words, look, cx))
                    .into_any_element(),
                Change::Added => {
                    let hue = look.color(token::STEP_ADVANCED);
                    labelled(
                        div()
                            .id(SharedString::from(format!("pull-draft-added:{at}")))
                            .min_w_0()
                            .pl(look.length(token::SPACE_3))
                            .pr(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .border_l(edge * 3.)
                            .border_color(hue)
                            .rounded_r(look.length(token::RADIUS_SM))
                            .bg(hue.opacity(0.10))
                            .tooltip(tip("Added by the draft"))
                            .child(self.prose(format!("pull-draft:added:{at}"), &words, look, cx)),
                        Role::Group,
                        "Added by the draft",
                    )
                    .into_any_element()
                }
                Change::Removed => {
                    let hue = look.color(token::STEP_FAILED);
                    labelled(
                        div()
                            .id(SharedString::from(format!("pull-draft-removed:{at}")))
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .pl(look.length(token::SPACE_3))
                            .pr(look.length(token::SPACE_2))
                            .py(look.length(token::SPACE_1))
                            .border_l(edge * 3.)
                            .border_color(hue)
                            .rounded_r(look.length(token::RADIUS_SM))
                            .bg(hue.opacity(0.08))
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .text_color(look.color(token::FG_MUTED))
                            .line_through()
                            .tooltip(tip("Taken out by the draft"))
                            .children(lines.into_iter().filter(|line| !line.trim().is_empty()).map(|line| div().min_w_0().child(SharedString::from(line)))),
                        Role::Group,
                        "Taken out by the draft",
                    )
                    .into_any_element()
                }
            }
        });
        div().min_w_0().flex().flex_col().gap(look.length(token::SPACE_2)).children(runs).into_any_element()
    }

    /// Approve, Edit and Note; or the open box with its own send and Cancel; and what Fleet refused.
    fn draft_foot(&self, panel: &PullPanel, draft: &PullRequestDraft, look: &Look, cx: &Context<Self>) -> AnyElement {
        let pane = &panel.draft;
        let on = !pane.sending;
        let said = pane.said.clone().map(|why| labelled(div().id("pull-draft-said").text_size(look.length(token::TEXT_XS)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()), Role::Alert, why));
        let row = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2));
        let row = match &pane.writing {
            None => {
                let (fixed, noted) = (draft.text.clone(), String::new());
                row.child(labelled(
                    small_button(look, "pull-draft-approve", "Approve", true, on)
                        .tooltip(tip("The pull request takes this description"))
                        .when(on, |button| button.on_click(cx.listener(|this, _, window, cx| this.draft_answer(Some(0), None, None, "Approved", window, cx)))),
                    Role::Button,
                    "Approve the description",
                ))
                .child(labelled(
                    small_button(look, "pull-draft-edit", "Edit", false, on)
                        .tooltip(tip("Change the words, then approve them"))
                        .when(on, |button| button.on_click(cx.listener(move |this, _, window, cx| this.draft_box(Writing::Edit, fixed.clone(), window, cx)))),
                    Role::Button,
                    "Edit the description",
                ))
                .child(labelled(
                    small_button(look, "pull-draft-note", "Note", false, on)
                        .tooltip(tip("Send it back to the Job with a note"))
                        .when(on, |button| button.on_click(cx.listener(move |this, _, window, cx| this.draft_box(Writing::Note, noted.clone(), window, cx)))),
                    Role::Button,
                    "Note on the description",
                ))
            }
            Some((writing, state)) => {
                let writing = *writing;
                let typed = state.read(cx).value().to_string();
                let ready = on && !typed.trim().is_empty();
                let (word, hint, name) = match writing {
                    Writing::Edit => ("Approve with fixes", "The pull request takes the description as you fixed it", "Draft text"),
                    Writing::Note => ("Send back", "The Job gets your note and writes the description again", "Note for the Job"),
                };
                let field = div()
                    .id("pull-draft-box")
                    .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                        if action.secondary {
                            cx.stop_propagation();
                            this.draft_commit(window, cx);
                        }
                    }))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_SUNKEN))
                    .child(Textarea::new(state).appearance(false));
                let (reveal, scroll) = (pane.reveal.clone(), panel.scroll.clone());
                // Scrolls the panel the least that brings the box and its buttons into view.
                let bring = canvas(
                    move |at, window, _| {
                        if !reveal.replace(false) {
                            return;
                        }
                        let view = scroll.bounds();
                        let mut offset = scroll.offset();
                        if at.bottom() > view.bottom() {
                            offset.y -= (at.bottom() - view.bottom()).min(at.top() - view.top());
                        } else if at.top() < view.top() {
                            offset.y += view.top() - at.top();
                        } else {
                            return;
                        }
                        scroll.set_offset(point(offset.x, offset.y.min(px(0.)).max(-scroll.max_offset().y)));
                        window.refresh();
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full();
                return div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap(look.length(token::SPACE_2))
                    .child(bring)
                    .child(labelled(field, Role::TextInput, name))
                    .child(
                        row.child(labelled(
                            small_button(look, "pull-draft-send", word, true, ready)
                                .tooltip(tip(format!("{hint} \u{b7} \u{2318}\u{21a9}")))
                                .when(ready, |button| button.on_click(cx.listener(|this, _, window, cx| this.draft_commit(window, cx)))),
                            Role::Button,
                            word,
                        ))
                        .child(labelled(
                            small_button(look, "pull-draft-cancel", "Cancel", false, on).when(on, |button| button.on_click(cx.listener(|this, _, _, cx| this.draft_cancel(cx)))),
                            Role::Button,
                            "Cancel",
                        )),
                    )
                    .children(said)
                    .into_any_element();
            }
        };
        div().flex().flex_col().gap(look.length(token::SPACE_2)).child(row).children(said).into_any_element()
    }

    /// Edit or Note: a box under the draft, holding what was typed in it before or `start`.
    fn draft_box(&mut self, writing: Writing, start: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.pull_panel().and_then(PullPanel::page).and_then(|page| page.draft.clone()) else { return };
        let key = kept_key(&draft, writing);
        let kept = self.draft_words(&key);
        let state = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 12));
        state.update(cx, |state, cx| state.set_value(if kept.trim().is_empty() { start } else { kept }, window, cx));
        let typing = cx.subscribe_in(&state, window, move |this, state, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                let text = state.read(cx).value().to_string();
                this.keep_words(&key, &text, cx);
                cx.notify();
            }
        });
        if let Some(panel) = self.pull_panel_mut() {
            panel.draft.writing = Some((writing, state.clone()));
            panel.draft._typing = Some(typing);
            panel.draft.said = None;
            panel.draft.reveal.set(true);
        }
        state.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    /// Cancel: the box goes, and what was typed in it is forgotten.
    fn draft_cancel(&mut self, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel_mut() else { return };
        let Some(draft) = panel.page().and_then(|page| page.draft.clone()) else { return };
        let Some((writing, _)) = panel.draft.writing.take() else { return };
        panel.draft._typing = None;
        panel.draft.said = None;
        self.keep_words(&kept_key(&draft, writing), "", cx);
        cx.notify();
    }

    /// ⌘↩ or the box's own button: Approve with fixes from Edit, Send back from Note.
    fn draft_commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((writing, state)) = self.pull_panel().and_then(|panel| panel.draft.writing.clone()) else { return };
        let typed = state.read(cx).value().to_string();
        if typed.trim().is_empty() {
            return;
        }
        match writing {
            Writing::Edit => {
                let same = self.pull_panel().and_then(PullPanel::page).and_then(|page| page.draft.as_ref()).is_some_and(|draft| draft.text == typed);
                if same {
                    self.draft_answer(Some(0), None, None, "Approved", window, cx);
                } else {
                    self.draft_answer(None, None, Some(typed), "Approved with fixes", window, cx);
                }
            }
            Writing::Note => self.draft_answer(None, Some(typed.trim().to_string()), None, "Sent back", window, cx),
        }
    }

    /// Answers the draft's `draft:<id>` item for the Job that holds the pull request, then reads the page again.
    fn draft_answer(&mut self, choice: Option<u32>, text: Option<String>, fixed: Option<String>, word: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let Some(draft) = panel.page().and_then(|page| page.draft.clone()) else { return };
        if panel.draft.sending {
            return;
        }
        let pull = panel.pull.clone();
        let writing = panel.draft.writing.as_ref().map(|(writing, _)| *writing);
        if let Some(panel) = self.pull_panel_mut() {
            panel.draft.sending = true;
            panel.draft.said = None;
        }
        cx.notify();
        let body = ipc::encode(&AnswerWaiting { session_id: SessionId::carried(pull.job.clone()), item_id: format!("draft:{}", draft.draft), choice, text, mode: None, fixed });
        let number = pull.number;
        self.post(
            "/sessions/waiting/answer".to_string(),
            body,
            window,
            cx,
            Box::new(move |this, answer, window, cx| {
                let Some(panel) = this.pull_panel_mut().filter(|panel| panel.pull.number == number) else { return };
                panel.draft.sending = false;
                match answer {
                    Ok(_) => {
                        panel.draft.writing = None;
                        panel.draft._typing = None;
                        panel.draft.current = false;
                        panel.draft.settled = Some((draft.draft.clone(), draft.revision, word));
                        if let Some(writing) = writing {
                            this.keep_words(&kept_key(&draft, writing), "", cx);
                        }
                        this.read_pull_page(window, cx);
                    }
                    Err(why) => panel.draft.said = Some(why.into()),
                }
                cx.notify();
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(runs: &[(Change, Vec<String>)]) -> Vec<(Change, Vec<&str>)> {
        runs.iter().map(|(kind, lines)| (*kind, lines.iter().map(String::as_str).collect())).collect()
    }

    #[test]
    fn a_draft_reads_as_the_lines_it_keeps_drops_and_adds() {
        let before = "Shows where it stands.\n\n- reads placeOf\n- draws nothing when out";
        let after = "Shows where it stands, on its plate.\n\n- reads placeOf\n- draws nothing when out\n- marks its own row";
        assert_eq!(
            lines(&changes(before, after)),
            vec![
                (Change::Removed, vec!["Shows where it stands."]),
                (Change::Added, vec!["Shows where it stands, on its plate."]),
                (Change::Same, vec!["", "- reads placeOf", "- draws nothing when out"]),
                (Change::Added, vec!["- marks its own row"]),
            ]
        );
    }

    #[test]
    fn the_same_words_are_one_run_and_trailing_spaces_do_not_count() {
        assert_eq!(lines(&changes("One\nTwo  \n", "One\nTwo\n")), vec![(Change::Same, vec!["One", "Two"])]);
    }
}
