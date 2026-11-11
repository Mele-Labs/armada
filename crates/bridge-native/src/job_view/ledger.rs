//! The rows a Job's lists share (`LedgerRow.tsx`, `SessionLayers.tsx`'s `Row`): the kind's glyph in
//! the kind's hue and a title that wraps to two lines, then a line of chips: the path or type in mono,
//! the kind's word, and the state as a filled chip. A row with somewhere to go is a button; one
//! without is read. Subagents and the Ledger draw them.

use std::rc::Rc;

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Window};
use ipc::{AttachmentState, ToolSubagent, ToolSubagentState};

use super::rows::{glyph, mono, tip, Size};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// Where a row's press goes.
pub(super) type Opens = Rc<dyn Fn(&mut JobView, &mut Window, &mut Context<JobView>)>;

/// A row's state, as the ledger's marks draw it (`SessionLedgerTone`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tone {
    Pending,
    Passed,
    Failed,
    Running,
    Done,
    Escalated,
    Piloted,
    Stopped,
}

impl Tone {
    /// Its glyph (`TONE`), the hue of its chip (`MARK_HUE`) and its word (`TONE_WORD`).
    fn look(self) -> (&'static str, Token, &'static str) {
        match self {
            Tone::Pending => ("shield-ellipsis", token::STATUS_AWAITING_REVIEW, "Waiting"),
            Tone::Passed => ("shield-check", token::STEP_ADVANCED, "Passed"),
            Tone::Failed => ("shield-x", token::STEP_FAILED, "Failed"),
            Tone::Running => ("circle-dot", token::STEP_RUNNING, "Running"),
            Tone::Done => ("check", token::STEP_ADVANCED, "Done"),
            Tone::Escalated => ("megaphone", token::STATUS_AWAITING_REVIEW, "Waiting"),
            Tone::Piloted => ("terminal", token::STATUS_AWAITING_REVIEW, "Piloted"),
            Tone::Stopped => ("power", token::STATUS_NOT_STARTED, "Stopped"),
        }
    }
}

/// A row's state: its tone, what its tooltip says, and a word that stands in for the tone's own.
pub(super) struct Mark {
    pub tone: Tone,
    pub said: String,
    pub word: Option<String>,
}

/// One row of a Job's lists (`ArtifactRow`).
pub(super) struct Line {
    pub key: String,
    /// The artifact kind, which gives the glyph its hue.
    pub kind: &'static str,
    pub glyph: &'static str,
    /// What the glyph is: its tooltip, and the kind's word on the second line.
    pub said: &'static str,
    pub title: String,
    pub sub: Option<String>,
    pub words: Vec<String>,
    pub mark: Option<Mark>,
    pub opens: Option<Opens>,
}

/// Each kind's own hue (`ledger-row.css`).
fn hue(kind: &str) -> Token {
    match kind {
        "pull_request" => token::STEP_RUNNING,
        "issue" | "link" | "session" => token::ACCENT,
        "commit" => token::STEP_ADVANCED,
        "page" => token::TOOL_LOOK,
        "file" => token::TOOL_CHANGE,
        "doc" => token::STATUS_HANDED_IN,
        "sketch" => token::STATUS_ESCALATED,
        "subagent" | "studio" => token::STATUS_REJECTED,
        "command" => token::TOOL_RUN,
        _ => token::FG_MUTED,
    }
}

/// The subagent a ledger row names, as the thread's Agent call would carry it, for its sheet.
pub(super) fn subagent_of(one: &ipc::Attachment) -> ToolSubagent {
    let standing = one.state == AttachmentState::Standing;
    ToolSubagent {
        id: one.target.clone(),
        agent_type: one.detail.get("type").filter(|kind| !kind.is_empty()).cloned(),
        task: one.detail.get("task").or_else(|| one.detail.get("description")).cloned().unwrap_or_else(|| "Subagent".to_string()),
        latest: None,
        state: if standing { ToolSubagentState::Running } else { ToolSubagentState::Finished },
        since: one.since.clone(),
        ended: (!standing).then(|| one.changed_at.clone()),
    }
}

impl JobView {
    /// A Job's subagents as rows, running ones first, then in the ledger's order (`artifactsOf`).
    pub(super) fn subagent_lines(&self, id: &str, cx: &gpui::App) -> Vec<Line> {
        let Some(record) = self.record(id, cx) else { return Vec::new() };
        let mut held: Vec<&ipc::Attachment> = record.attachments.iter().filter(|one| one.kind == "subagent").collect();
        held.sort_by_key(|one| one.state != AttachmentState::Standing);
        held.into_iter()
            .map(|one| {
                let subagent = subagent_of(one);
                let running = subagent.state == ToolSubagentState::Running;
                let (tone, said) = if running { (Tone::Running, "Running") } else { (Tone::Done, "Done") };
                let title = subagent.task.clone();
                let sub = subagent.agent_type.clone();
                let opens: Opens = Rc::new(move |this: &mut JobView, window: &mut Window, cx: &mut Context<JobView>| this.read_subagent(subagent.clone(), window, cx));
                Line {
                    key: format!("subagent:{}", one.target),
                    kind: "subagent",
                    glyph: "split",
                    said: "Subagent",
                    title,
                    sub,
                    words: Vec::new(),
                    mark: Some(Mark { tone, said: said.to_string(), word: None }),
                    opens: Some(opens),
                }
            })
            .collect()
    }

    /// Subagents: every subagent the Job started, as one list with no heading (`SubagentsLayer`).
    /// None, and nothing is drawn.
    pub(super) fn pane_subagents(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let lines = self.subagent_lines(id, cx);
        if lines.is_empty() {
            return None;
        }
        Some(labelled(div().id("pane-subagents").flex().flex_col().children(self.ledger_rows(lines, look, cx)), Role::List, "Subagents").into_any_element())
    }

    /// The Ledger: everything attached to the Job, by kind (`LedgerBody`).
    pub(super) fn pane_ledger(&self, _id: &SharedString, _look: &Look, _cx: &Context<Self>) -> Option<AnyElement> {
        None
    }

    /// Rows one under another, a hairline between them.
    pub(super) fn ledger_rows(&self, lines: Vec<Line>, look: &Look, cx: &Context<Self>) -> Vec<AnyElement> {
        lines.into_iter().enumerate().map(|(at, line)| self.ledger_row(line, at == 0, look, cx)).collect()
    }

    fn ledger_row(&self, line: Line, first: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let ink = look.color(hue(line.kind));
        let chip = |words: SharedString| div().flex_none().px(look.length(token::SPACE_1)).rounded(look.length(token::RADIUS_SM)).child(words);
        let sub = line.sub.map(|sub| {
            div()
                .min_w_0()
                .truncate()
                .px(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .bg(look.color(token::FG_DEFAULT).opacity(0.08))
                .font_family(mono(look))
                .child(SharedString::from(sub))
        });
        let words = line.words.into_iter().map(|word| div().min_w_0().truncate().child(SharedString::from(word)));
        let kind_word = chip(line.said.into()).bg(ink.opacity(0.14)).text_color(ink).font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)));
        let fact = line.mark.map(|mark| {
            let (icon, tone, word) = mark.tone.look();
            let fact = look.color(tone);
            div()
                .id(SharedString::from(format!("ledger-fact:{}", line.key)))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .pl(look.length(token::SPACE_1))
                .pr(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_SM))
                .bg(fact.opacity(0.14))
                .text_color(fact)
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .tooltip(tip(mark.said.clone()))
                .child(glyph(look, icon, Size::Small, fact))
                .child(SharedString::from(mark.word.unwrap_or_else(|| word.to_string())))
        });
        let lead = div()
            .id(SharedString::from(format!("ledger-glyph:{}", line.key)))
            .flex_none()
            .w(look.length(token::SPACE_3))
            .h(look.length(token::LEADING_XS))
            .flex()
            .items_center()
            .justify_center()
            .tooltip(tip(line.said))
            .child(glyph(look, line.glyph, Size::Small, ink));
        let main = div()
            .id(SharedString::from(format!("ledger-row:{}", line.key)))
            .flex_1()
            .min_w_0()
            .flex()
            .items_start()
            .gap(look.length(token::SPACE_2))
            .p(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .child(lead)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(div().min_w_0().line_clamp(2).text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(line.title.clone())))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(look.length(token::SPACE_2))
                            .min_w_0()
                            .overflow_hidden()
                            .text_size(look.length(token::TEXT_2XS))
                            .line_height(look.length(token::LEADING_2XS))
                            .text_color(look.color(token::FG_MUTED))
                            .whitespace_nowrap()
                            .children(sub)
                            .children(words)
                            .child(kind_word)
                            .children(fact),
                    ),
            );
        let main = match line.opens {
            Some(opens) => labelled(
                main.cursor_pointer().hover(|style| style.bg(look.color(token::BG_HOVER))).on_click(cx.listener(move |this, _, window, cx| opens(this, window, cx))),
                Role::Button,
                line.title,
            )
            .into_any_element(),
            None => main.into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .when(!first, |row| row.border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)))
            .child(main)
            .into_any_element()
    }
}
