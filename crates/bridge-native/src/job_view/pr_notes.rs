//! His notes on a pull request's lines, in its panel's Files tab (`shared-panel/job-notes-lines.tsx` and
//! `job-notes.ts`). Fleet keeps them on this Mac and posts nothing to the forge. A note is started from a
//! line's gutter, saved with ⌘↩ or Save note, and stands under its line. The notes bar sends the unsent
//! ones to a Job as one message, then Fleet records them as sent, so the Job can resolve each by its id.

use std::rc::Rc;

use chrono::Utc;
use gpui::{div, prelude::*, px, AnyElement, Context, Entity, Role, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, Textarea, TextareaState};
use ipc::{NoteSide, NoteStatus, PullRequestNote, PullRequestNotes, SavePullRequestNote, SendPullRequestNotes};

use super::pr_panel::{icon_button, small_button, PullPanel};
use super::pulls::Pull;
use super::rows::{glyph, tip, Size};
use super::JobView;
use crate::diff::{self, DiffLine, Gutter, Slots};
use crate::dropdown::Entry;
use crate::labels::labelled;
use crate::look::{token, Look};

const LABEL: &str = "Note for the job";

/// The box a note is written in, under the line it is about. `editing` names the note it changes.
pub(super) struct NoteBox {
    pub path: String,
    pub index: usize,
    pub line: u32,
    pub side: NoteSide,
    pub editing: Option<String>,
    pub input: Entity<TextareaState>,
    /// The words are kept under this while they are being written (`keep`).
    keep: String,
    _typing: Subscription,
}

/// What the Job reads: each note numbered, with the file and line the forge names it by and the id to
/// resolve it with (`messageOf`).
fn message_of(number: u64, notes: &[&PullRequestNote]) -> String {
    let mut lines = vec![
        format!("Notes on #{number} from the owner, kept on his Mac and not posted to the pull request. When you have dealt with a note, call resolve_pull_request_note with its id."),
        String::new(),
    ];
    for (at, note) in notes.iter().enumerate() {
        let removed = if note.side == NoteSide::Left { " on the removed line" } else { "" };
        lines.push(format!("{}. {}:{}{removed} [{}]: {}", at + 1, note.path, note.line, note.id, note.body));
    }
    lines.join("\n")
}

fn unsent(note: &PullRequestNote) -> bool {
    note.sent_to.is_none()
}

impl JobView {
    fn pull_notes<'a>(&'a self, panel: &PullPanel) -> &'a [PullRequestNote] {
        panel.patches_key().and_then(|key| self.panes.notes.get(&key)).map_or(&[], Vec::as_slice)
    }

    /// `GET /pull_request_notes/:manifest/:number`, on opening and on every entry to Files. On mock data
    /// the scenario's notes, once, so what he writes stays.
    pub(super) fn want_pull_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let (Some(key), Some(manifest), number) = (panel.patches_key(), panel.pull.manifest.clone(), panel.pull.number) else { return };
        if self.fleet.is_none() {
            if !self.panes.notes.contains_key(&key) {
                self.panes.notes.insert(key, crate::mock::pull_notes(&format!("{manifest}#{number}")).unwrap_or_default());
            }
            return;
        }
        self.get(
            format!("/pull_request_notes/{}/{number}", super::wire::encoded(&manifest)),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Ok(read) = answer.and_then(|bytes| ipc::decode::<PullRequestNotes>("PullRequestNotes", &bytes).map_err(|why| why.to_string())) {
                    this.panes.notes.insert(key, read.notes);
                    cx.notify();
                }
            }),
        );
    }

    /// A line's gutter was pressed, or Edit on a note: the box opens under the line with the words kept for it.
    fn open_note_box(&mut self, path: String, index: usize, place: (u32, NoteSide), editing: Option<(String, String)>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let Some(key) = panel.patches_key() else { return };
        let keep = match &editing {
            Some((id, _)) => format!("job-notes:edit:{id}"),
            None => format!("job-notes:{key}:{path}:{index}"),
        };
        let kept = self.draft_words(&keep);
        let words = if kept.is_empty() { editing.as_ref().map(|(_, body)| body.clone()).unwrap_or_default() } else { kept };
        let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8));
        input.update(cx, |input, cx| {
            input.set_value(words, window, cx);
            input.focus(window, cx);
        });
        let kept_key = keep.clone();
        // ⌘↩ is the box's own: the text area takes it and says so (`sendsOn`).
        let typing = cx.subscribe_in(&input, window, move |this, input, event: &InputEvent, window, cx| match event {
            InputEvent::Change => {
                let text = input.read(cx).value().to_string();
                this.keep_words(&kept_key, &text, cx);
                cx.notify();
            }
            InputEvent::PressEnter { secondary: true, .. } => this.save_note(window, cx),
            _ => {}
        });
        if let Some(panel) = self.pull_panel_mut() {
            panel.note_box = Some(NoteBox { path, index, line: place.0, side: place.1, editing: editing.map(|(id, _)| id), input, keep, _typing: typing });
        }
        cx.notify();
    }

    fn cancel_note(&mut self, cx: &mut Context<Self>) {
        if let Some(panel) = self.pull_panel_mut() {
            panel.note_box = None;
        }
        cx.notify();
    }

    /// Save note, or ⌘↩ in its box: a new note is made on its line, or the note being changed takes the words
    /// (`POST /pull_request_notes/:manifest/:number`). A blank box saves nothing.
    fn save_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let Some(open) = panel.note_box.as_ref() else { return };
        let body = open.input.read(cx).value().trim().to_string();
        if body.is_empty() {
            return;
        }
        let (Some(key), Some(manifest), number) = (panel.patches_key(), panel.pull.manifest.clone(), panel.pull.number) else { return };
        let (path, line, side, editing, keep) = (open.path.clone(), open.line, open.side, open.editing.clone(), open.keep.clone());
        self.keep_words(&keep, "", cx);
        if let Some(panel) = self.pull_panel_mut() {
            panel.note_box = None;
        }
        if self.fleet.is_none() {
            let notes = self.panes.notes.entry(key).or_default();
            match editing {
                Some(id) => {
                    if let Some(note) = notes.iter_mut().find(|note| note.id == id) {
                        note.body = body;
                    }
                }
                None => {
                    let id = format!("note-{}", notes.len() + 1);
                    let repository = manifest.clone();
                    notes.push(PullRequestNote { id, repository, number: u32::try_from(number).unwrap_or_default(), path, line, side, body, status: NoteStatus::Open, sent_to: None, sent_title: None, sent_at: None, created_at: Utc::now().to_rfc3339() });
                }
            }
            cx.notify();
            return;
        }
        let save = match editing {
            Some(id) => SavePullRequestNote { id: Some(id), path: None, line: None, side: None, body },
            None => SavePullRequestNote { id: None, path: Some(path), line: Some(line), side: Some(side), body },
        };
        self.post(
            format!("/pull_request_notes/{}/{number}", super::wire::encoded(&manifest)),
            ipc::encode(&save),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Ok(note) = answer.and_then(|bytes| ipc::decode::<PullRequestNote>("PullRequestNote", &bytes).map_err(|why| why.to_string())) {
                    let notes = this.panes.notes.entry(key).or_default();
                    match notes.iter_mut().find(|one| one.id == note.id) {
                        Some(one) => *one = note,
                        None => notes.push(note),
                    }
                    cx.notify();
                }
            }),
        );
    }

    /// Remove the note (`POST /pull_request_notes/:manifest/:number/:id/remove`); Fleet answers the notes left.
    fn remove_note(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let (Some(key), Some(manifest), number) = (panel.patches_key(), panel.pull.manifest.clone(), panel.pull.number) else { return };
        if self.fleet.is_none() {
            self.panes.notes.entry(key).or_default().retain(|note| note.id != id);
            cx.notify();
            return;
        }
        self.post_bare(
            format!("/pull_request_notes/{}/{number}/{}/remove", super::wire::encoded(&manifest), super::wire::encoded(&id)),
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Ok(read) = answer.and_then(|bytes| ipc::decode::<PullRequestNotes>("PullRequestNotes", &bytes).map_err(|why| why.to_string())) {
                    this.panes.notes.insert(key, read.notes);
                    cx.notify();
                }
            }),
        );
    }

    /// Send to job: the unsent notes go to the Job as one message, as its own box sends one, and once it took
    /// them Fleet records them as sent to it (`sendNotes`, `recordSent`).
    fn send_notes(&mut self, to: String, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let Some(panel) = self.pull_panel() else { return };
        let (Some(key), Some(manifest), number) = (panel.patches_key(), panel.pull.manifest.clone(), panel.pull.number) else { return };
        let unsent: Vec<&PullRequestNote> = self.pull_notes(panel).iter().filter(|note| unsent(note)).collect();
        if unsent.is_empty() {
            return;
        }
        let ids: Vec<String> = unsent.iter().map(|note| note.id.clone()).collect();
        let text = message_of(number, &unsent);
        let target = to.clone();
        self.send_to_job(to, text, format!("ledger:pr:{number}"), window, cx, move |this, window, cx| {
            if this.fleet.is_none() {
                let at = Utc::now().to_rfc3339();
                for note in this.panes.notes.entry(key).or_default().iter_mut().filter(|note| ids.contains(&note.id)) {
                    note.sent_to = Some(ipc::SessionId::carried(target.clone()));
                    note.sent_title = Some(title.clone());
                    note.sent_at = Some(at.clone());
                }
                cx.notify();
                return;
            }
            this.record_sent(key, manifest, number, ids, target, title, window, cx);
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn record_sent(&mut self, key: String, manifest: String, number: u64, note_ids: Vec<String>, to: String, title: String, window: &mut Window, cx: &mut Context<Self>) {
        let body = ipc::encode(&SendPullRequestNotes { note_ids, session_id: ipc::SessionId::carried(to), title });
        self.post(
            format!("/pull_request_notes/{}/{number}/sent", super::wire::encoded(&manifest)),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| {
                if let Ok(read) = answer.and_then(|bytes| ipc::decode::<PullRequestNotes>("PullRequestNotes", &bytes).map_err(|why| why.to_string())) {
                    this.panes.notes.insert(key, read.notes);
                    cx.notify();
                }
            }),
        );
    }

    /// What one file's lines carry in Files: the gutter that starts a note, and under a line its notes and
    /// the open box (`useLineNotes`). None where the pull request has no repository to keep notes under.
    pub(super) fn note_slots(&self, panel: &PullPanel, path: &str, lines: &[DiffLine], look: &Look, cx: &Context<Self>) -> Slots {
        if panel.pull.manifest.is_none() {
            return Slots::default();
        }
        let places = diff::places(lines);
        let view = cx.entity().downgrade();
        let at_path = path.to_string();
        let press_places = places.clone();
        let press: Rc<dyn Fn(usize, bool, &mut Window, &mut gpui::App)> = Rc::new(move |index, _, window, cx| {
            let Some(Some((line, left))) = press_places.get(index).copied() else { return };
            let side = if left { NoteSide::Left } else { NoteSide::Right };
            let path = at_path.clone();
            let _ = view.update(cx, |this, cx| this.open_note_box(path, index, (line as u32, side), None, window, cx));
        });
        let texts: Vec<String> = lines.iter().map(|line| line.text.trim().to_string()).collect();
        let name: Rc<dyn Fn(usize) -> String> = Rc::new(move |index| format!("{LABEL} on {}", texts.get(index).map_or("this line", String::as_str)));
        let mut slots = Slots { gutter: Some(Gutter { label: LABEL.into(), name: Some(name), press }), ..Slots::default() };
        let notes = self.pull_notes(panel);
        for (index, place) in places.iter().enumerate() {
            let Some((line, left)) = *place else { continue };
            let side = if left { NoteSide::Left } else { NoteSide::Right };
            let here: Vec<&PullRequestNote> = notes
                .iter()
                .filter(|note| note.path == path && u64::from(note.line) == line && note.side == side && (panel.notes_resolved || note.status == NoteStatus::Open))
                .collect();
            let writing = panel.note_box.as_ref().filter(|open| open.path == path && open.index == index);
            if here.is_empty() && writing.is_none() {
                continue;
            }
            if writing.is_some() {
                slots.picked = Some(index);
            }
            let mut under: Vec<AnyElement> = Vec::new();
            for note in here {
                match panel.note_box.as_ref().filter(|open| open.editing.as_deref() == Some(note.id.as_str())) {
                    Some(open) => under.push(self.note_box_view(open, look, cx)),
                    None => under.push(self.note_card(note, look, cx)),
                }
            }
            if let Some(open) = writing.filter(|open| open.editing.is_none()) {
                under.push(self.note_box_view(open, look, cx));
            }
            slots.under.insert(index, div().flex().flex_col().gap(look.length(token::SPACE_2)).children(under).into_any_element());
        }
        slots
    }

    /// One note under its line (`NoteCard`): a dashed card in his own hue, what has become of it, its words,
    /// Edit until it is sent and Remove.
    fn note_card(&self, note: &PullRequestNote, look: &Look, cx: &Context<Self>) -> AnyElement {
        let to = note.sent_title.clone().unwrap_or_else(|| "the job".into());
        let done = note.status == NoteStatus::Done;
        let (icon, said): (&'static str, String) = if done {
            ("check", format!("Resolved by {to}"))
        } else if note.sent_to.is_some() {
            ("check", format!("Sent to {to}"))
        } else {
            ("notebook", "Kept on this Mac, not sent yet".into())
        };
        let who = if done { format!("Resolved by {to}") } else { LABEL.to_string() };
        let mark = labelled(
            div().id(SharedString::from(format!("note-mark:{}", note.id))).flex_none().tooltip(tip(said.clone())).child(glyph(look, icon, Size::Small, look.color(token::FG_MUTED))),
            Role::Image,
            said,
        );
        let edit = unsent(note).then(|| {
            let (id, body) = (note.id.clone(), note.body.clone());
            let (path, line, side) = (note.path.clone(), note.line, note.side);
            labelled(
                icon_button(look, SharedString::from(format!("note-edit:{}", note.id)), "pencil", "Edit the note".into()).on_click(cx.listener(move |this, _, window, cx| {
                    this.open_note_box(path.clone(), usize::MAX, (line, side), Some((id.clone(), body.clone())), window, cx);
                })),
                Role::Button,
                "Edit the note",
            )
        });
        let id = note.id.clone();
        let remove = labelled(
            icon_button(look, SharedString::from(format!("note-remove:{}", note.id)), "x", "Remove the note".into()).on_click(cx.listener(move |this, _, window, cx| this.remove_note(id.clone(), window, cx))),
            Role::Button,
            "Remove the note",
        );
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .pl(look.length(token::SPACE_3))
            .pr(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .text_color(look.color(token::FG_MUTED))
            .child(mark)
            .child(div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).font_weight(gpui::FontWeight(look.number(token::WEIGHT_MEDIUM))).child(who))
            .children(edit)
            .child(remove);
        labelled(
            div()
                .id(SharedString::from(format!("note:{}", note.id)))
                .flex()
                .flex_col()
                .min_w_0()
                .rounded(look.length(token::RADIUS_MD))
                .border(look.length(token::BORDER_WIDTH))
                .border_dashed()
                .border_color(look.color(token::YOU_EDGE))
                .bg(look.color(token::BG_RAISED))
                .child(div().relative().child(div().absolute().inset_0().rounded(look.length(token::RADIUS_MD)).bg(look.color(token::YOU).opacity(0.08))).child(head))
                .child(div().px(look.length(token::SPACE_3)).pb(look.length(token::SPACE_2)).child(self.prose(format!("note-body:{}", note.id), &note.body, look, cx))),
            Role::Article,
            LABEL,
        )
        .into_any_element()
    }

    /// The box under a line (`LineComposer`): three lines to write in, Cancel and Save note. ⌘↩ saves; Enter
    /// starts a new line.
    fn note_box_view(&self, open: &NoteBox, look: &Look, cx: &Context<Self>) -> AnyElement {
        let blank = open.input.read(cx).value().trim().is_empty();
        let field = div()
            .id("note-box-field")
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .child(Textarea::new(&open.input).appearance(false));
        let cancel = labelled(small_button(look, "note-box-cancel", "Cancel", false, true).on_click(cx.listener(|this, _, _, cx| this.cancel_note(cx))), Role::Button, "Cancel");
        let save = small_button(look, "note-box-save", "Save note", false, !blank)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED))
            .text_color(look.color(token::FG_DEFAULT))
            .tooltip(tip("Save note \u{b7} \u{2318}\u{21a9}"))
            .when(!blank, |button| button.on_click(cx.listener(|this, _, window, cx| this.save_note(window, cx))));
        labelled(
            div()
                .id("note-box")
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_2))
                .pt(look.length(token::SPACE_2))
                .pb(look.length(token::SPACE_3))
                .child(labelled(field, Role::TextInput, LABEL))
                .child(div().flex().justify_end().gap(look.length(token::SPACE_2)).child(cancel).child(labelled(save, Role::Button, "Save note"))),
            Role::Form,
            LABEL,
        )
        .into_any_element()
    }

    /// The Job that holds the pull request, by id and title, where one does (`owner`).
    fn note_holder(&self, pull: &Pull, cx: &gpui::App) -> Option<(String, String)> {
        if !pull.held() {
            return None;
        }
        let title = self.record(&pull.job, cx).and_then(|record| record.title.clone()).unwrap_or_else(|| pull.job.clone());
        Some((pull.job.clone(), title))
    }

    /// The running Jobs the notes can go to instead, by id and title (`others`).
    fn note_others(&self, holder: Option<&str>, cx: &gpui::App) -> Vec<(String, String)> {
        self.list
            .read(cx)
            .records()
            .iter()
            .filter(|record| record.ended_at.is_none() && Some(record.id.as_str()) != holder)
            .take(super::pop::NUMBERED.len())
            .map(|record| (record.id.as_str().to_string(), record.title.clone().unwrap_or_else(|| record.id.as_str().to_string())))
            .collect()
    }

    /// What waits to be sent, and where (`NotesBar`): the face sends to the Job that holds the pull request,
    /// and the caret lists the other running Jobs. Where none holds it the face is off and the caret alone is live.
    pub(super) fn notes_bar(&self, panel: &PullPanel, pull: &Pull, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let count = self.pull_notes(panel).iter().filter(|note| unsent(note)).count();
        let holder = self.note_holder(pull, cx);
        let others = self.note_others(holder.as_ref().map(|(id, _)| id.as_str()), cx);
        if count == 0 || (holder.is_none() && others.is_empty()) {
            return None;
        }
        let notes = if count == 1 { "1 note".to_string() } else { format!("{count} notes") };
        let said = match &holder {
            Some((_, title)) => format!("{notes} for {title}"),
            None => format!("{notes} kept, choose a job"),
        };
        let face = labelled(
            small_button(look, "notes-send", "Send to job", true, holder.is_some())
                .rounded_r(px(0.))
                .when_some(holder.clone(), |button, (id, title)| button.on_click(cx.listener(move |this, _, window, cx| this.send_notes(id.clone(), title.clone(), window, cx)))),
            Role::Button,
            "Send to job",
        );
        let menu_others = others.clone();
        let caret = labelled(
            small_button(look, "notes-send-menu", "", true, !others.is_empty())
                .px(look.length(token::SPACE_2))
                .rounded_l(px(0.))
                .border_l(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::BG_BASE))
                .child(glyph(look, "chevron-down", Size::Small, look.color(token::TEXT_ON_ACCENT)))
                .when(!others.is_empty(), |button| {
                    button.on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
                        let entries: Vec<Entry> = menu_others
                            .iter()
                            .zip(super::pop::NUMBERED)
                            .map(|((_, title), id)| Entry::Act { id, label: title.clone().into(), icon: None, danger: false })
                            .collect();
                        let picks = menu_others.clone();
                        this.open_button_menu(
                            "Send to a job",
                            entries,
                            event.position(),
                            None,
                            move |this, id, window, cx| {
                                if let Some((to, title)) = super::pop::NUMBERED.iter().position(|one| *one == id).and_then(|at| picks.get(at)) {
                                    this.send_notes(to.clone(), title.clone(), window, cx);
                                }
                            },
                            window,
                            cx,
                        );
                    }))
                }),
            Role::Button,
            "Send to a job",
        );
        Some(
            labelled(
                div()
                    .id("notes-bar")
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .rounded(look.length(token::RADIUS_MD))
                    .border(px(1.))
                    .border_dashed()
                    .border_color(look.color(token::YOU_EDGE))
                    .text_size(look.length(token::TEXT_XS))
                    .child(div().min_w_0().child(said))
                    .child(div().flex().flex_none().child(face).child(caret)),
                Role::Group,
                "Notes for the job",
            )
            .into_any_element(),
        )
    }

    /// Folds the notes a Job resolved away, or draws them again (`ResolvedToggle`). Drawn only where one has.
    pub(super) fn notes_resolved_toggle(&self, panel: &PullPanel, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let resolved = self.pull_notes(panel).iter().filter(|note| note.status == NoteStatus::Done).count();
        if resolved == 0 {
            return None;
        }
        let shown = panel.notes_resolved;
        let said = if shown { "Hide resolved".to_string() } else { format!("Show resolved ({resolved})") };
        Some(
            div()
                .flex()
                .child(labelled(
                    small_button(look, "notes-resolved", &said, false, true).on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(panel) = this.pull_panel_mut() {
                            panel.notes_resolved = !shown;
                        }
                        cx.notify();
                    })),
                    Role::Button,
                    said.clone(),
                ))
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, path: &str, line: u32, side: NoteSide, body: &str) -> PullRequestNote {
        PullRequestNote { id: id.into(), repository: "a/b".into(), number: 7, path: path.into(), line, side, body: body.into(), status: NoteStatus::Open, sent_to: None, sent_title: None, sent_at: None, created_at: String::new() }
    }

    #[test]
    fn the_message_numbers_each_note_with_its_file_line_and_id() {
        let one = note("n1", "src/a.rs", 4, NoteSide::Right, "Name it.");
        let two = note("n2", "src/b.rs", 9, NoteSide::Left, "Keep this.");
        let said = message_of(7, &[&one, &two]);
        assert!(said.starts_with("Notes on #7 from the owner, kept on his Mac and not posted to the pull request."));
        assert!(said.contains("1. src/a.rs:4 [n1]: Name it."));
        assert!(said.contains("2. src/b.rs:9 on the removed line [n2]: Keep this."));
    }
}
