//! The reader as it is drawn, and the keys it answers (`reader.rs` holds what it remembers).
//!
//! | Part | Draws |
//! |---|---|
//! | Header band | The kind's tile, the title at text-xl, the revision picker, where it stands, the Edit switch, the For line, and Dismiss, Ask for changes and Approve. A page in its Job's tab is answered in the review sheet alone: its header says it waits, with Open |
//! | Contents | The headings, on the left where there is room, else a menu above the words |
//! | Words | A draft's blocks with a note's words marked in his colour, a moved block with a running edge |
//! | Gutter | Each note pinned beside its block; a table takes the whole width and its notes stand under it; in a narrow reader every note stands under its paragraph |
//!
//! Keys, with the reader holding the focus: N a note on the words picked, J and K between notes, Enter into the one
//! on, Backspace drops it, A C D answer, E edits, R the revisions, T the contents, S shares a page.

use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, Hsla, KeyDownEvent, Role, SharedString, Window};
use gpui_component::input::{Enter, Textarea};
use gpui_component::text::TextView;
use ipc::{ArtifactBy, ArtifactForm, ItemHow};

use super::acts::{kbd, SHEET};
use super::band_card::{state_chip, tile};
use super::reader::{BlockKind, Reader, Room, Subject};
use super::row_tabs::RowTab;
use super::rows::{mono, tip};
use super::sketch_card::{act, link_chip, mixed, Kind};
use super::JobView;
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look, Token};

/// What a press or a key on the reader does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RAct {
    Approve,
    Ask,
    Dismiss,
    Archive,
    Edit,
    Save,
    Discard,
}

/// What the revision picker reads: the word, two spaces and the number, never "of" the newest.
pub(super) fn revision_said(shown: u32) -> String {
    format!("Revision  {shown}")
}

/// The revisions a menu can name: ids a menu entry may carry.
const REVS: [&str; 24] = ["r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15", "r16", "r17", "r18", "r19", "r20", "r21", "r22", "r23", "r24"];
const HEADS: [&str; 40] = [
    "h0", "h1", "h2", "h3", "h4", "h5", "h6", "h7", "h8", "h9", "h10", "h11", "h12", "h13", "h14", "h15", "h16", "h17", "h18", "h19", "h20", "h21", "h22", "h23", "h24", "h25", "h26", "h27", "h28", "h29", "h30", "h31",
    "h32", "h33", "h34", "h35", "h36", "h37", "h38", "h39",
];

/// The key a reader answers with while nothing in it holds the caret: the act, or what to do with a note.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum RKey {
    Act(RAct),
    Note,
    Next,
    Prev,
    Into,
    Drop,
    Revisions,
    Contents,
    Share,
}

/// What a bare key means to the reader. A, C and D answer only while it asks, E edits only where it can.
pub(super) fn key_of(key: &str, asking: bool, editable: bool, page: bool) -> Option<RKey> {
    Some(match key {
        "a" if asking => RKey::Act(RAct::Approve),
        "c" if asking => RKey::Act(RAct::Ask),
        "d" => RKey::Act(if asking { RAct::Dismiss } else { RAct::Archive }),
        "e" if editable => RKey::Act(RAct::Edit),
        "n" => RKey::Note,
        "j" => RKey::Next,
        "k" => RKey::Prev,
        "enter" => RKey::Into,
        "backspace" | "delete" => RKey::Drop,
        "r" => RKey::Revisions,
        "t" => RKey::Contents,
        "s" if page => RKey::Share,
        _ => return None,
    })
}

impl JobView {
    /// The body of a draft's or a page's tab: the reader.
    pub(super) fn reader_body(&self, id: &SharedString, tab: &RowTab, look: &Look, cx: &Context<Self>) -> AnyElement {
        let name = super::reader::reader_id(tab.kind, &tab.key);
        let (Some(held), Some(subject)) = (self.reader_of(id, &name), self.subject_of(id, tab.kind, &tab.key, cx)) else {
            return div().id("row-tab-body").track_focus(&self.row_focus).size_full().into_any_element();
        };
        let room = held.room();
        let editing = held.edit.is_some();
        let earlier = held.viewing.is_some_and(|at| at != subject.newest);
        let (head, foot) = self.reader_head(id, &name, held, &subject, earlier, look, cx);
        let strip = (!room.rail()).then(|| self.reader_strip(id, &name, held, look, cx)).flatten();
        let rail = room.rail().then(|| self.reader_rail(id, &name, held, look, cx));
        let words = self.reader_words(id, &name, held, &subject, room, look, cx);
        let (me, width) = (cx.weak_entity(), held.width.clone());
        let measure = gpui::canvas(
            move |bounds, _, cx| {
                if (f32::from(width.get()) - f32::from(bounds.size.width)).abs() > 1. {
                    width.set(bounds.size.width);
                    let me = me.clone();
                    cx.defer(move |cx| {
                        me.update(cx, |_, cx| cx.notify()).ok();
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        // A page in its Job's tab is answered in the review sheet alone, so A, C and D answer nothing here.
        let pointer = tab.kind == super::row_tabs::RowKind::Document;
        let (job, key_name, asking, editable, page) = (id.clone(), name.clone(), subject.asking && !earlier && !pointer, !subject.closed && !earlier, subject.form == ArtifactForm::Page);
        let waits = pointer && subject.asking;
        let (job_c, name_c) = (id.clone(), name.clone());
        let body = div()
            .id("reader-body")
            .relative()
            .flex_1()
            .min_h_0()
            .flex()
            .children(rail)
            .child(div().flex_1().min_w_0().flex().flex_col().children(strip).child(words))
            .child(measure);
        let ask = held.ask.as_ref().filter(|_| subject.asking && !earlier && !pointer).map(|(area, _)| self.reader_ask_box(id, &name, held, area, look, cx));
        let said = held.said.clone().map(|why| div().px(look.length(token::SPACE_4)).py(look.length(token::SPACE_2)).child(super::acts::refusal(look, "reader", why)));
        labelled(
            div()
                .id("row-tab-body")
                .track_focus(&self.row_focus)
                .size_full()
                .flex()
                .flex_col()
                .bg(look.color(token::BG_BASE))
                .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let (key, mods) = (event.keystroke.key.as_str(), &event.keystroke.modifiers);
                    if this.reader_of(&job_c, &name_c).is_some_and(|held| held.edit.is_some()) {
                        if key == "escape" && !mods.modified() {
                            cx.stop_propagation();
                            this.reader_edit_leave(&job_c, &name_c, window, cx);
                            window.focus(&this.row_focus, cx);
                        } else if key == "s" && mods.platform && !mods.shift && !mods.alt {
                            cx.stop_propagation();
                            this.reader_act(&job_c, &name_c, RAct::Save, window, cx);
                        }
                    }
                }))
                .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    let mods = &event.keystroke.modifiers;
                    // Words typed into a note or a block are theirs: the reader hears a bare key anywhere else in it.
                    if mods.modified() || this.reader_of(&job, &key_name).is_none_or(|held| held.typing(window, cx)) {
                        return;
                    }
                    let Some(act) = key_of(event.keystroke.key.as_str(), asking, editable, page) else { return };
                    if waits && act == RKey::Act(RAct::Archive) {
                        return;
                    }
                    cx.stop_propagation();
                    this.reader_key(&job, &key_name, act, window, cx);
                }))
                .child(head)
                .child(body)
                .children(ask)
                .children(said)
                .children(foot)
                .when(editing, |root| root),
            Role::Region,
            format!("Reader: {}", subject.title),
        )
        .into_any_element()
    }

    /// The header band: where it stands fills it, and his colour fills it while he edits. Two lines that never wrap: the
    /// kind, the title, the revision and where it stands, with the answers at the right; then the For line cut to one
    /// line, with what else is his to press at the right. The title gives way first.
    #[allow(clippy::too_many_arguments)]
    ///
    /// In the focus view the acts leave the header for a bar pinned at the foot: the revision picker, the notes toggle and what
    /// he answers with. The header keeps the kind, the title, where it stands and the For line.
    fn reader_head(&self, id: &SharedString, name: &str, held: &Reader, subject: &Subject, earlier: bool, look: &Look, cx: &Context<Self>) -> (AnyElement, Option<AnyElement>) {
        let pinned = false;
        let editing = held.edit.is_some();
        let ink: Token = if editing { token::YOU } else { subject.stand.ink() };
        let (fill, edge) = if editing { (mixed(look, ink, 0.16), look.color(ink).opacity(0.45)) } else { (mixed(look, ink, 0.18), look.color(ink).opacity(0.4)) };
        let shown = held.viewing.unwrap_or(subject.newest);
        let last = subject.revisions.last();
        let yours = last.filter(|last| last.by == ArtifactBy::Owner && last.number == subject.newest && !editing).map(|last| format!("r{} is your edit", last.number));
        let picker = self.reader_picker(id, name, shown, look, cx);
        let switch = (!subject.closed && !earlier && matches!(subject.form, ArtifactForm::Draft | ArtifactForm::Page)).then(|| self.reader_switch(id, name, editing, look, cx));
        let (words, discard, save) = if editing { self.reader_edit_acts(id, name, held, look, cx) } else { (None, None, None) };
        // A page in its Job's tab is answered in the review sheet alone: the tab says it waits, with Open to the sheet.
        let pointer = held.kind == super::row_tabs::RowKind::Document;
        // While Ask for changes is open under the words, its box holds the answer and the header holds none.
        let asking = subject.asking && !earlier && !editing && held.ask.is_none() && !pointer;
        let go = |which: RAct, label: &'static str, key: &'static str, said: &'static str, kind: Kind| -> AnyElement {
            let (job, name_c) = (id.clone(), name.to_string());
            labelled(
                act(look, format!("reader-{label}:{name}"), label, kind, key, true)
                    .tooltip(tip(said))
                    .on_click(cx.listener(move |this, _, window, cx| this.reader_act(&job, &name_c, which, window, cx))),
                Role::Button,
                label,
            )
            .into_any_element()
        };
        let ask = asking.then(|| go(RAct::Ask, "Ask for changes", "C", "Send it back with your notes", Kind::Secondary));
        let approve = asking.then(|| go(RAct::Approve, "Approve", "A", "Approve it as it stands, with your notes", Kind::Approve));
        // The answers keep their order, Discard and Save while he edits, then Ask for changes and Approve.
        let answers: Vec<AnyElement> = [discard, save, ask, approve].into_iter().flatten().collect();
        let (picker_at_top, picker_pinned) = if pinned { (None, Some(picker)) } else { (Some(picker), None) };
        let (answers_at_top, answers_pinned) = if pinned { (Vec::new(), answers) } else { (answers, Vec::new()) };
        let title = SharedString::from(subject.title.clone());
        let top = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .child(tile(look, subject.form))
            .child(
                div()
                    .id(SharedString::from(format!("reader-title:{name}")))
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(look.length(token::TEXT_XL))
                    .line_height(look.length(token::LEADING_XL))
                    .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                    .text_color(look.color(token::FG_DEFAULT))
                    .tooltip(tip(title.clone()))
                    .child(title),
            )
            .children(picker_at_top)
            .children((!editing).then(|| state_chip(look, format!("reader-state:{name}"), subject.stand, subject.words.as_deref(), None)))
            .children(answers_at_top);
        // What he can do besides answering: Edit, and putting it away.
        let away = if subject.asking && pointer {
            None
        } else if subject.asking && !earlier && !editing {
            Some(go(RAct::Dismiss, "Dismiss", "D", "Put it away with no answer", Kind::Ghost))
        } else if !subject.archived && !earlier && !editing && !subject.asking {
            Some(go(RAct::Archive, "Archive", "D", "Fold it away from your list", Kind::Ghost))
        } else {
            None
        };
        let line = subject.for_line.as_ref().filter(|line| line.text.as_deref().is_some_and(|text| !text.trim().is_empty()) || !line.links.is_empty());
        let line = if editing { words } else { line.map(|line| self.reader_for_line(name, line, look)) };
        let notes = self.reader_notes_button(id, name, held.notes_on, look, cx);
        // Share, last of the acts as Classic's head has it, with what a save came to before it.
        let share = (subject.form == ArtifactForm::Page && !editing).then(|| self.reader_share_button(id, name, look, cx));
        let shared = held.shared.clone().map(|said| {
            labelled(
                div().id(SharedString::from(format!("reader-shared:{name}"))).flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(said.clone()),
                Role::Status,
                said,
            )
        });
        let (under_acts, foot_acts) = if pinned { ((None, None, None), (switch, away, Some(notes))) } else { ((switch, away, Some(notes)), (None, None, None)) };
        let under = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(div().flex_1().min_w_0().overflow_hidden().children(line))
            .children(yours.map(|said| super::band_card::yours_pill(look, format!("reader-yours:{name}"), said)))
            .children(under_acts.0)
            .children(under_acts.1)
            .children(under_acts.2)
            .children(shared)
            .children(share);
        let waits = (pointer && subject.asking && !earlier && !editing).then(|| {
            let page = held.key.clone();
            let said: &'static str = "This page waits on your Approve";
            let open = labelled(
                act(look, format!("reader-waits-open:{name}"), "Open", Kind::Secondary, "", true)
                    .tooltip(tip("Answer it in the review sheet"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.set_needs(false, None, window, cx);
                        this.open_review_at(Some(&page), window, cx);
                    })),
                Role::Button,
                "Open",
            );
            labelled(
                div()
                    .id(SharedString::from(format!("reader-waits:{name}")))
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).text_color(look.color(token::FG_DEFAULT)).child(said))
                    .child(open),
                Role::Group,
                said,
            )
        });
        let foot = pinned.then(|| {
            labelled(
                div()
                    .id(SharedString::from(format!("reader-foot:{name}")))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_3))
                    .bg(look.color(token::BG_BASE))
                    .border_t(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_SUBTLE))
                    .children(foot_acts.0)
                    .children(foot_acts.1)
                    .children(foot_acts.2)
                    .children(picker_pinned)
                    .children(answers_pinned),
                Role::Group,
                "Reader acts",
            )
            .into_any_element()
        });
        let head = div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .bg(fill)
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .child(top)
            .child(under)
            .children(waits)
            .into_any_element();
        (head, foot)
    }

    /// The For line on one line, cut with an ellipsis where it does not fit, its whole words in a tooltip.
    fn reader_for_line(&self, name: &str, line: &ipc::ArtifactFor, look: &Look) -> AnyElement {
        let text = line.text.clone().unwrap_or_default();
        let whole = SharedString::from(text.clone());
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .child(
                div()
                    .id(SharedString::from(format!("reader-for:{name}")))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .gap(look.length(token::SPACE_1))
                    .when(!text.trim().is_empty(), |one| one.tooltip(tip(whole.clone())))
                    .child(div().flex_none().font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM))).text_color(look.color(token::FG_DEFAULT)).child("For:"))
                    .child(div().flex_1().min_w_0().truncate().child(whole)),
            )
            .children(line.links.iter().map(|link| link_chip(look, link)))
            .into_any_element()
    }

    /// The notes' margin on or off, as the registry gives a document's head: the glyph shows what a press does.
    fn reader_notes_button(&self, id: &SharedString, name: &str, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, name_c) = (id.clone(), name.to_string());
        // A page's margin holds the talk about it; a draft's holds his notes.
        let page = self.reader_of(id, name).is_some_and(|held| held.kind == super::row_tabs::RowKind::Document);
        let said = match (page, on) {
            (true, true) => "Hide the margin",
            (true, false) => "Show the margin",
            (false, true) => "Hide the notes margin",
            (false, false) => "Show the notes margin",
        };
        labelled(
            super::head::icon_button(look, "reader-notes", if on { "panel-right-close" } else { "panel-right-open" }, format!("{said}, N").into(), on, false)
                .id(SharedString::from(format!("reader-notes:{name}")))
                .on_click(cx.listener(move |this, _, _, cx| this.reader_toggle_notes(&job, &name_c, cx))),
            Role::Switch,
            if page { "Margin" } else { "Notes" },
        )
        .into_any_element()
    }

    /// While he edits: what he is editing, Discard and Save.
    fn reader_edit_acts(&self, id: &SharedString, name: &str, held: &Reader, look: &Look, cx: &Context<Self>) -> (Option<AnyElement>, Option<AnyElement>, Option<AnyElement>) {
        let revision = held.viewing.map_or_else(|| held.marked.unwrap_or(1), |at| at);
        let saving = held.saving;
        let (job, name_c, job2, name2) = (id.clone(), name.to_string(), id.clone(), name.to_string());
        let you = look.color(token::YOU);
        let discard = labelled(
            act(look, format!("reader-discard:{name}"), "Discard", Kind::Ghost, "esc", true)
                .tooltip(tip("Leave your words out"))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_act(&job, &name_c, RAct::Discard, window, cx))),
            Role::Button,
            "Discard",
        );
        let ink = look.color(token::FG_INVERSE);
        let save = labelled(
            div()
                .id(SharedString::from(format!("reader-save:{name}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .gap(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH) * 2.)
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(you)
                .bg(you)
                .text_color(ink)
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .when(!saving, |button| button.cursor_pointer().hover(|style| style.opacity(0.9)))
                .when(saving, |button| button.opacity(0.4))
                .tooltip(tip("Keep your words as the next revision"))
                .child("Save")
                .child(
                    div()
                        .flex_none()
                        .px(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_SM))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(ink.opacity(0.5))
                        .font_family(mono(look))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .child("\u{2318}S"),
                )
                .when(!saving, |button| button.on_click(cx.listener(move |this, _, window, cx| this.reader_act(&job2, &name2, RAct::Save, window, cx)))),
            Role::Button,
            "Save",
        );
        let words = div().min_w_0().truncate().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(format!("Editing r{revision}"))).into_any_element();
        (Some(words), Some(discard.into_any_element()), Some(save.into_any_element()))
    }

    /// The Edit switch: a track and a knob, in his colour while it is on.
    fn reader_switch(&self, id: &SharedString, name: &str, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, name_c) = (id.clone(), name.to_string());
        let you = look.color(token::YOU);
        let track = div()
            .relative()
            .flex_none()
            .w(look.length(token::SPACE_6) + look.length(token::SPACE_1))
            .h(look.length(token::SPACE_4))
            .rounded_full()
            .bg(if on { you } else { look.color(token::BORDER_STRONG) })
            .child(
                div()
                    .absolute()
                    .top(look.length(token::SPACE_1) / 2.)
                    .when(on, |knob| knob.right(look.length(token::SPACE_1) / 2.))
                    .when(!on, |knob| knob.left(look.length(token::SPACE_1) / 2.))
                    .size(look.length(token::SPACE_3))
                    .rounded_full()
                    .bg(look.color(if on { token::FG_INVERSE } else { token::FG_DEFAULT })),
            );
        labelled(
            div()
                .id(SharedString::from(format!("reader-edit:{name}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .cursor_pointer()
                .text_size(look.length(token::TEXT_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(if on { token::FG_DEFAULT } else { token::FG_MUTED }))
                .tooltip(tip(if on { "Leave Edit" } else { "Change its words" }))
                .child(track)
                .child("Edit")
                .child(kbd(look, "E"))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_act(&job, &name_c, RAct::Edit, window, cx))),
            Role::Switch,
            "Edit",
        )
        .into_any_element()
    }

    /// `Revision  3` and a chevron, over a menu of every revision, newest first: the numbers are in the menu.
    fn reader_picker(&self, id: &SharedString, name: &str, shown: u32, look: &Look, cx: &Context<Self>) -> AnyElement {
        let (job, name_c) = (id.clone(), name.to_string());
        let said = revision_said(shown);
        labelled(
            div()
                .id(SharedString::from(format!("reader-revisions:{name}")))
                .flex_none()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_1))
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_STRONG))
                .bg(look.color(token::BG_OVERLAY))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                .text_color(look.color(token::FG_DEFAULT))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Read an earlier revision"))
                .child(SharedString::from(said.clone()))
                .child(gpui::svg().path(crate::assets::icon_path("chevron-down")).flex_none().size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_revisions(&job, &name_c, window.mouse_position(), false, window, cx))),
            Role::Button,
            said,
        )
        .into_any_element()
    }

    /// The revisions as a menu: each with who kept it and when.
    pub(super) fn reader_revisions(&mut self, id: &SharedString, name: &str, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let Some(subject) = self.subject_of(id, held.kind, &held.key, cx) else { return };
        let shown = held.viewing.unwrap_or(subject.newest);
        let has_before = held.before.is_some();
        let mut entries = Vec::new();
        for revision in subject.revisions.iter().rev().take(REVS.len()) {
            let Some(slot) = REVS.get(revision.number as usize - 1) else { continue };
            let said = super::band_card::revision_said(revision.number, revision.by, revision.at.as_str());
            // A draft's thread keeps its newest words and the ones they replaced; a page serves every revision.
            let kept = subject.form == ArtifactForm::Page || revision.number == subject.newest || (has_before && revision.number + 1 == subject.newest);
            if kept {
                entries.push(Entry::Choice { id: slot, label: said.into(), on: revision.number == shown });
            } else {
                entries.push(Entry::Off { label: said.into(), hint: "Only the revision before is kept to read" });
            }
        }
        let (job, name_c) = (id.clone(), name.to_string());
        let back = Some(self.row_focus.clone());
        self.open_pop("Revisions", entries, at, from_keys, back, move |this, picked, window, cx| {
            if let Some(number) = picked.strip_prefix('r').and_then(|number| number.parse::<u32>().ok()) {
                this.reader_view(&job, &name_c, number, window, cx);
            }
        }, window, cx);
    }

    /// The contents as a menu, where there is no room for them at the left.
    fn reader_strip(&self, id: &SharedString, name: &str, held: &Reader, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let heads = self.reader_heads(id, held, cx);
        if heads.is_empty() {
            return None;
        }
        let on = self.reader_on_head(held, &heads);
        let said = heads.get(on).map(|(_, _, words)| words.clone()).unwrap_or_default();
        let (job, name_c) = (id.clone(), name.to_string());
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_1))
                .border_b(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .bg(look.color(token::BG_SUNKEN))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("reader-contents:{name}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .px(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::BORDER_STRONG))
                        .bg(look.color(token::BG_OVERLAY))
                        .text_size(look.length(token::TEXT_2XS))
                        .text_color(look.color(token::FG_DEFAULT))
                        .cursor_pointer()
                        .tooltip(tip("Go to a heading"))
                        .child(gpui::svg().path(crate::assets::icon_path("file-text")).flex_none().size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)))
                        .child(SharedString::from(said))
                        .child(gpui::svg().path(crate::assets::icon_path("chevron-down")).flex_none().size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_MUTED)))
                        .on_click(cx.listener(move |this, _, window, cx| this.reader_contents(&job, &name_c, window.mouse_position(), false, window, cx))),
                    Role::Button,
                    "Contents",
                ))
                .into_any_element(),
        )
    }

    /// The contents as a menu of the headings.
    pub(super) fn reader_contents(&mut self, id: &SharedString, name: &str, at: gpui::Point<gpui::Pixels>, from_keys: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let heads = self.reader_heads(id, held, cx);
        let entries: Vec<Entry> = heads.iter().zip(HEADS.iter()).map(|((_, _, words), slot)| Entry::Act { id: slot, label: words.clone().into(), icon: None, danger: false }).collect();
        if entries.is_empty() {
            return;
        }
        let (job, name_c) = (id.clone(), name.to_string());
        let back = Some(self.row_focus.clone());
        self.open_pop("Contents", entries, at, from_keys, back, move |this, picked, _, cx| {
            if let Some(at) = picked.strip_prefix('h').and_then(|at| at.parse::<usize>().ok()) {
                this.reader_go(&job, &name_c, at, cx);
            }
        }, window, cx);
    }

    /// The heading the words are at: the last that has reached the top of the column.
    fn reader_on_head(&self, held: &Reader, heads: &[(usize, u8, String)]) -> usize {
        if held.kind == super::row_tabs::RowKind::Document {
            return held.at_head.min(heads.len().saturating_sub(1));
        }
        let top = held.scroll.bounds().top();
        let mut on = 0;
        for (at, (block, _, _)) in heads.iter().enumerate() {
            if held.scroll.bounds_for_item(*block).is_some_and(|bounds| bounds.top() <= top + px(8.)) {
                on = at;
            }
        }
        on
    }

    /// The contents rail: the headings, the one the words are at drawn on.
    fn reader_rail(&self, id: &SharedString, name: &str, held: &Reader, look: &Look, cx: &Context<Self>) -> AnyElement {
        let heads = self.reader_heads(id, held, cx);
        let on = self.reader_on_head(held, &heads);
        let entries = heads.into_iter().enumerate().map(|(at, (_, level, words))| {
            let (job, name_c) = (id.clone(), name.to_string());
            labelled(
                div()
                    .id(SharedString::from(format!("reader-head:{name}:{at}")))
                    .pl(look.length(token::SPACE_2) + look.length(token::SPACE_2) * f32::from(level.saturating_sub(2)))
                    .pr(look.length(token::SPACE_2))
                    .py(look.length(token::SPACE_1) + look.length(token::BORDER_WIDTH))
                    .rounded(look.length(token::RADIUS_MD))
                    .text_size(look.length(token::TEXT_XS))
                    .cursor_pointer()
                    .when(at == on, |row| row.bg(look.color(token::SURFACE_ROW_SELECTED)).text_color(look.color(token::FG_DEFAULT)))
                    .when(at != on, |row| row.text_color(look.color(token::FG_MUTED)).hover(|style| style.bg(look.color(token::BG_HOVER))))
                    .child(SharedString::from(words.clone()))
                    .on_click(cx.listener(move |this, _, _, cx| this.reader_go(&job, &name_c, at, cx))),
                Role::Button,
                format!("Go to {words}"),
            )
        });
        labelled(
            div()
                .id(SharedString::from(format!("reader-rail:{name}")))
                .flex_none()
                .w(look.length(token::SPACE_12) * 3. + look.length(token::SPACE_4))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_4))
                .bg(look.color(token::BG_SUNKEN))
                .border_r(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BORDER_SUBTLE))
                .overflow_y_scroll()
                .child(div().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child("Contents"))
                .children(entries),
            Role::Region,
            "Contents",
        )
        .into_any_element()
    }

    /// The words: a draft's blocks, each with its notes pinned beside it.
    #[allow(clippy::too_many_arguments)]
    fn reader_words(&self, id: &SharedString, name: &str, held: &Reader, subject: &Subject, room: Room, look: &Look, cx: &Context<Self>) -> AnyElement {
        if subject.form == ArtifactForm::Page {
            return self.reader_page_words(id, name, held, subject, room, look, cx);
        }
        let mine = subject.revisions.last().is_some_and(|last| last.by == ArtifactBy::Owner && last.number == subject.newest);
        let editing = held.edit.as_ref();
        let changed = editing.map(|edit| edit.changed(cx)).unwrap_or_default();
        let first_text = held.blocks.iter().position(|one| one.block.kind == BlockKind::Text);
        let rows = held.blocks.iter().enumerate().map(|(at, one)| {
            let notes: Vec<AnyElement> = held
                .notes
                .iter()
                .enumerate()
                .filter(|(_, note)| held.notes_on && note.block == one.block.id)
                .map(|(place, note)| self.reader_pin(id, name, place, note, held.on == Some(place), look, cx))
                .collect();
            let words = match (editing, &one.block.kind) {
                (Some(edit), _) => self.reader_edit_block(name, at, edit.areas.get(at), changed.get(at).copied().unwrap_or(false), look),
                (None, BlockKind::Heading(level)) => heading(look, *level, &one.block.source),
                (None, _) => match &one.state {
                    Some(state) => {
                        let view = TextView::new(state)
                            .w_full()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(gpui::relative(1.6))
                            .text_color(look.color(token::FG_DEFAULT))
                            .selectable(true)
                            .on_link_click(|address, _, _, cx| cx.open_url(address));
                        div().w_full().child(view).into_any_element()
                    }
                    None => div().into_any_element(),
                },
            };
            let moved = held.moved.get(at).copied().unwrap_or(false) && editing.is_none();
            let words = if moved {
                labelled(
                    div()
                        .id(SharedString::from(format!("reader-moved:{name}:{at}")))
                        .w_full()
                        .pl(look.length(token::SPACE_3))
                        .ml(-(look.length(token::SPACE_3) + look.length(token::BORDER_WIDTH) * 3.))
                        .border_l(look.length(token::BORDER_WIDTH) * 3.)
                        .border_color(look.color(if mine { token::YOU } else { token::STATUS_RUNNING }))
                        .tooltip(tip(if mine { "Your words, kept as this revision" } else { "Changed since you last opened it" }))
                        .child(words),
                    Role::Group,
                    if mine { "Your words, kept as this revision" } else { "Changed since you last opened it" },
                )
                .into_any_element()
            } else {
                words
            };
            let margin = super::reader::notes_margin(held.notes_on, room, look);
            let wide = margin > px(0.) && editing.is_none() && one.block.kind != BlockKind::Table;
            let hint = (Some(at) == first_text && editing.is_none() && !subject.closed).then(|| self.reader_hint(look));
            let bounds = one.bounds.clone();
            let measure = gpui::canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {}).absolute().top_0().left_0().size_full();
            let block = div().relative().flex_1().min_w_0().child(words).child(measure);
            let row = div().flex().items_start().gap(look.length(token::SPACE_6)).w_full().child(block);
            let row = if wide {
                row.child(div().flex_none().w(margin - look.length(token::SPACE_6)).flex().flex_col().gap(look.length(token::SPACE_2)).children(notes).children(hint))
            } else if notes.is_empty() {
                row
            } else {
                row
            };
            let under = (held.notes_on && !wide && editing.is_none()).then(|| div().flex().flex_col().gap(look.length(token::SPACE_2)).pt(look.length(token::SPACE_2)).children(notes_under(&held.notes, &one.block.id, |place, note| self.reader_pin(id, name, place, note, held.on == Some(place), look, cx))));
            let under = under.filter(|_| !room.gutter() || one.block.kind == BlockKind::Table);
            let mut stack = div().flex().flex_col().w_full().gap(look.length(token::SPACE_1)).child(row);
            if let Some(under) = under {
                stack = stack.child(under);
            }
            labelled(
                stack.id(SharedString::from(format!("reader-block:{name}:{at}"))).mb(look.length(token::SPACE_2)),
                Role::Group,
                format!("Reader words {}", one.block.id),
            )
            .into_any_element()
        });
        div()
            .id(SharedString::from(format!("reader-scroll:{name}")))
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&held.scroll)
            .px(look.length(token::SPACE_8))
            .py(look.length(token::SPACE_6))
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                // A selection keeps the keys with the reader, so N is the next press.
                window.focus(&this.row_focus, cx);
            }))
            .children(rows)
            .into_any_element()
    }

    /// The words are marked and N is how a note is made: said once, in the first gutter.
    fn reader_hint(&self, look: &Look) -> AnyElement {
        div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(kbd(look, "N"))
            .child("add a note")
            .into_any_element()
    }

    /// One note pinned in the gutter: the words it points at, and a box for what he says of them.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn reader_pin(&self, id: &SharedString, name: &str, place: usize, note: &super::reader::NoteBox, on: bool, look: &Look, cx: &Context<Self>) -> AnyElement {
        let you: Hsla = look.color(token::YOU);
        let quote = note.quote.trim();
        let said = if quote.is_empty() { "Note".to_string() } else { format!("on \u{201c}{}\u{201d}", cut(quote, 80)) };
        let (job, name_c, job2, name2) = (id.clone(), name.to_string(), id.clone(), name.to_string());
        let (job3, name3) = (id.clone(), name.to_string());
        let area = note.area.clone();
        let blank = note.area.read(cx).value().trim().is_empty();
        labelled(
            div()
                .id(SharedString::from(format!("reader-note:{name}:{place}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .border_l(look.length(token::BORDER_WIDTH) * 3.)
                .border_color(you)
                .rounded_r(look.length(token::RADIUS_MD))
                .bg(mixed(look, token::YOU, if on { 0.16 } else { 0.09 }))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_DEFAULT))
                .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        window.focus(&this.row_focus, cx);
                        if blank {
                            if let Some(held) = this.reader_mut(&job, &name_c) {
                                held.on = Some(place);
                            }
                            this.reader_drop_note(&job, &name_c, window, cx);
                        }
                    }
                }))
                .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                    if action.secondary {
                        cx.stop_propagation();
                        this.reader_key(&job2, &name2, RKey::Act(RAct::Ask), window, cx);
                    }
                }))
                .child(
                    div()
                        .id(SharedString::from(format!("reader-note-quote:{name}:{place}")))
                        .min_w_0()
                        .truncate()
                        .cursor_pointer()
                        .text_size(look.length(token::TEXT_2XS))
                        .text_color(look.color(token::FG_SUBTLE))
                        .tooltip(tip("Show it in the words"))
                        .child(SharedString::from(said.clone()))
                        .on_click(cx.listener(move |this, _, _, cx| this.reader_show_note(&job3, &name3, place, cx))),
                )
                .child(labelled(
                    div().id(SharedString::from(format!("reader-note-box:{name}:{place}"))).key_context(crate::keymap::TEXT_BOX).child(Textarea::new(&area).appearance(false).aria_label("Note")),
                    Role::TextInput,
                    "Note",
                )),
            Role::Group,
            format!("Note {said}"),
        )
        .into_any_element()
    }

    /// Ask for changes, open under the words: the notes already on the words, then what should change, Cancel and Send back.
    fn reader_ask_box(&self, id: &SharedString, name: &str, held: &super::reader::Reader, area: &gpui::Entity<gpui_component::input::TextareaState>, look: &Look, cx: &Context<Self>) -> AnyElement {
        let warn = look.color(token::STATUS_AWAITING_REVIEW);
        let noted: Vec<AnyElement> = held
            .kept_notes(cx)
            .into_iter()
            .filter(|note| !note.text.trim().is_empty())
            .enumerate()
            .map(|(at, note)| {
                let said = format!("{} \u{b7} {}", at + 1, note.text.trim());
                div()
                    .id(SharedString::from(format!("reader-ask-note:{name}:{at}")))
                    .max_w(px(320.))
                    .truncate()
                    .px(look.length(token::SPACE_2))
                    .rounded_full()
                    .border(look.length(token::BORDER_WIDTH))
                    .border_color(look.color(token::BORDER_DEFAULT))
                    .bg(look.color(token::BG_RAISED))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_XS))
                    .text_color(look.color(token::FG_MUTED))
                    .tooltip(tip(said.clone()))
                    .child(SharedString::from(said))
                    .into_any_element()
            })
            .collect();
        let (job, job2, job3, job4) = (id.clone(), id.clone(), id.clone(), id.clone());
        let (name_a, name_b, name_c, name_d) = (name.to_string(), name.to_string(), name.to_string(), name.to_string());
        let cancel = labelled(
            act(look, format!("reader-ask-cancel:{name}"), "Cancel", Kind::Ghost, "Esc", true)
                .tooltip(tip("Close the box; your notes on the words stay"))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_ask_cancel(&job, &name_a, window, cx))),
            Role::Button,
            "Cancel",
        );
        let send = labelled(
            act(look, format!("reader-ask-send:{name}"), "Send back", Kind::Secondary, "\u{2318}\u{21a9}", true)
                .tooltip(tip("Send it back with these words and your notes"))
                .on_click(cx.listener(move |this, _, window, cx| this.reader_key(&job2, &name_b, RKey::Act(RAct::Ask), window, cx))),
            Role::Button,
            "Send back",
        );
        labelled(
            div()
                .id(SharedString::from(format!("reader-ask:{name}")))
                .flex_none()
                .mx(look.length(token::SPACE_4))
                .my(look.length(token::SPACE_3))
                .p(look.length(token::SPACE_3))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_LG))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(warn.opacity(0.6))
                .bg(look.color(token::BG_SUNKEN))
                .capture_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && !event.keystroke.modifiers.modified() {
                        cx.stop_propagation();
                        this.reader_ask_cancel(&job3, &name_c, window, cx);
                    }
                }))
                .capture_action(cx.listener(move |this, action: &Enter, window, cx| {
                    if action.secondary {
                        cx.stop_propagation();
                        this.reader_key(&job4, &name_d, RKey::Act(RAct::Ask), window, cx);
                    }
                }))
                .when(!noted.is_empty(), |box_| box_.child(div().flex().flex_wrap().gap(look.length(token::SPACE_1)).children(noted)))
                .child(labelled(
                    div()
                        .id(SharedString::from(format!("reader-ask-box:{name}")))
                        .key_context(crate::keymap::TEXT_BOX)
                        .text_size(look.length(token::TEXT_SM))
                        .text_color(look.color(token::FG_DEFAULT))
                        .child(Textarea::new(area).appearance(false).aria_label("What should change?")),
                    Role::TextInput,
                    "What should change?",
                ))
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(send)),
            Role::Group,
            "Ask for changes",
        )
        .into_any_element()
    }

    /// A block's words as he edits them: its own box, underlined faintly where he can type, and marked in his colour once changed.
    fn reader_edit_block(&self, name: &str, at: usize, area: Option<&gpui::Entity<gpui_component::input::TextareaState>>, changed: bool, look: &Look) -> AnyElement {
        let Some(area) = area else { return div().into_any_element() };
        let you = look.color(token::YOU);
        let label = format!("Words {}", at + 1);
        labelled(
            div()
                .id(SharedString::from(format!("reader-edit-block:{name}:{at}")))
                .key_context(crate::keymap::TEXT_BOX)
                .w_full()
                .px(look.length(token::SPACE_2))
                .py(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .border_l(look.length(token::BORDER_WIDTH) * 3.)
                .border_color(if changed { you } else { gpui::transparent_black() })
                .bg(if changed { you.opacity(0.12) } else { gpui::transparent_black() })
                .border_b(look.length(token::BORDER_WIDTH))
                .when(!changed, |block| block.border_color(look.color(token::ACCENT).opacity(0.4)))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(Textarea::new(area).appearance(false).aria_label(SharedString::from(label.clone()))),
            Role::TextInput,
            label,
        )
        .into_any_element()
    }

    /// A key the reader answers: the act, or what to do with the notes.
    pub(super) fn reader_key(&mut self, id: &SharedString, name: &str, key: RKey, window: &mut Window, cx: &mut Context<Self>) {
        match key {
            RKey::Act(act) => self.reader_act(id, name, act, window, cx),
            RKey::Note => self.reader_note_key(id, name, window, cx),
            RKey::Next => self.reader_move(id, name, true, cx),
            RKey::Prev => self.reader_move(id, name, false, cx),
            RKey::Into => self.reader_into_note(id, name, window, cx),
            RKey::Drop => self.reader_drop_note(id, name, window, cx),
            RKey::Revisions => {
                let at = self.reader_of(id, name).map(|held| held.width.get()).map_or_else(|| px(0.), |_| px(0.));
                let at = gpui::point(at + px(240.), px(96.));
                self.reader_revisions(id, name, at, true, window, cx);
            }
            RKey::Contents => {
                self.reader_contents(id, name, gpui::point(px(240.), px(96.)), true, window, cx);
            }
            RKey::Share => self.reader_share(id, name, window, cx),
        }
    }

    /// A press or a key on an act: it answers, or turns Edit on or off.
    pub(super) fn reader_act(&mut self, id: &SharedString, name: &str, act: RAct, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        let Some(subject) = self.subject_of(id, held.kind, &held.key, cx) else { return };
        let earlier = held.viewing.is_some_and(|at| at != subject.newest);
        match act {
            RAct::Approve if subject.asking && !earlier => self.reader_answer(id, name, ItemHow::Approve, subject.card_key.clone(), subject.item.clone(), window, cx),
            RAct::Ask if subject.asking && !earlier => self.reader_answer(id, name, ItemHow::SendBack, subject.card_key.clone(), subject.item.clone(), window, cx),
            RAct::Dismiss if subject.asking => {
                let body = ipc::encode(&ipc::DismissWaiting { session_id: ipc::SessionId::carried(id.to_string()), item_id: subject.item.clone(), reason: None, misread: false }).map_err(|why| format!("{why:?}"));
                self.settle(SHEET, subject.card_key.clone(), super::acts::Answer::Withdrawn, "/sessions/waiting/dismiss", body, window, cx);
            }
            RAct::Archive if !subject.asking && !subject.archived => self.archive_card(SHEET, &subject.card_key, &subject.artifact, window, cx),
            RAct::Edit | RAct::Save | RAct::Discard => self.reader_edit_act(id, name, act, &subject, window, cx),
            _ => {}
        }
    }
}

/// A heading as the reader sets it: the page's own `h1` at text-lg, an `h2` in the step colour, the rest muted.
fn heading(look: &Look, level: u8, source: &str) -> AnyElement {
    let words = super::reader::heading_words(source);
    let (size, line, ink) = match level {
        1 => (token::TEXT_LG, token::LEADING_LG, token::FG_DEFAULT),
        2 => (token::TEXT_SM, token::LEADING_SM, token::KIND_STEP),
        _ => (token::TEXT_SM, token::LEADING_SM, token::FG_MUTED),
    };
    labelled(
        div()
            .id(SharedString::from(format!("reader-heading:{words}")))
            .w_full()
            .mt(look.length(token::SPACE_3))
            .text_size(look.length(size))
            .line_height(look.length(line))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(ink))
            .child(SharedString::from(words.clone())),
        Role::Heading,
        words,
    )
    .into_any_element()
}

fn cut(words: &str, most: usize) -> String {
    let flat: String = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= most {
        return flat;
    }
    format!("{}\u{2026}", flat.chars().take(most).collect::<String>().trim_end())
}

/// The notes on one block, with their places among every note.
fn notes_under(notes: &[super::reader::NoteBox], block: &str, pin: impl Fn(usize, &super::reader::NoteBox) -> AnyElement) -> Vec<AnyElement> {
    notes.iter().enumerate().filter(|(_, note)| note.block == block).map(|(place, note)| pin(place, note)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_key_is_an_act_a_note_or_a_move_between_notes() {
        assert_eq!(key_of("n", true, true, false), Some(RKey::Note), "N makes a note whenever the reader holds the keys");
        assert_eq!(key_of("n", false, false, false), Some(RKey::Note));
        assert_eq!(key_of("a", true, true, false), Some(RKey::Act(RAct::Approve)));
        assert_eq!(key_of("c", true, true, false), Some(RKey::Act(RAct::Ask)));
        assert_eq!(key_of("a", false, true, false), None, "an answered thing is not answered again");
        assert_eq!(key_of("d", true, true, false), Some(RKey::Act(RAct::Dismiss)));
        assert_eq!(key_of("d", false, true, false), Some(RKey::Act(RAct::Archive)));
        assert_eq!(key_of("e", true, true, false), Some(RKey::Act(RAct::Edit)));
        assert_eq!(key_of("e", true, false, false), None, "a settled draft is not edited");
        assert_eq!(key_of("e", true, true, true), Some(RKey::Act(RAct::Edit)), "a page's words are edited in place");
        assert_eq!(key_of("e", false, false, true), None, "a settled page is not edited");
        assert_eq!((key_of("j", true, true, false), key_of("k", true, true, false)), (Some(RKey::Next), Some(RKey::Prev)));
        assert_eq!(key_of("enter", true, true, false), Some(RKey::Into));
        assert_eq!(key_of("backspace", true, true, false), Some(RKey::Drop));
        assert_eq!(key_of("x", true, true, false), None);
        assert_eq!(key_of("s", false, false, true), Some(RKey::Share), "S shares a page");
        assert_eq!(key_of("s", true, true, false), None, "a draft has no Share");
    }

    #[test]
    fn the_revision_picker_reads_the_word_two_spaces_and_the_number_never_of() {
        assert_eq!(revision_said(3), "Revision  3");
        assert!(!revision_said(12).contains(" of "));
    }

    #[test]
    fn the_notes_margin_takes_no_width_while_notes_are_off() {
        let look = crate::look::Look::dark();
        for room in [Room::Wide, Room::Medium, Room::Narrow] {
            assert_eq!(super::super::reader::notes_margin(false, room, &look), px(0.), "off, in a {room:?} reader");
        }
        assert!(super::super::reader::notes_margin(true, Room::Wide, &look) > px(0.), "on, it has its width");
        assert_eq!(super::super::reader::notes_margin(true, Room::Narrow, &look), px(0.), "no room, the notes stand under the paragraph");
    }

    #[test]
    fn the_menus_name_enough_revisions_and_headings() {
        assert_eq!(REVS[23], "r24");
        assert_eq!(HEADS[39], "h39");
        assert!(REVS.iter().enumerate().all(|(at, name)| name.strip_prefix('r').and_then(|n| n.parse::<usize>().ok()) == Some(at + 1)));
    }

    #[test]
    fn words_cut_for_a_pin_end_in_an_ellipsis_and_lose_their_breaks() {
        assert_eq!(cut("steps\nunder   the title", 80), "steps under the title");
        assert_eq!(cut("abcdefghij", 4), "abcd\u{2026}");
    }
}
