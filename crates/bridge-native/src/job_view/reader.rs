//! The reader: one place to read a draft or a page, in the Gutter look the owner approved on 14 Nov 2026.
//! A header band with the kind, the title, the revision, where it stands and the acts; the contents on the left; the
//! words in a column with his notes pinned in a gutter beside what they point at. **Making a note takes one key**:
//! select words and press N, and the caret is already in the note.
//!
//! This file holds what the reader remembers and what its acts do; `reader_draw.rs` draws it. A draft's words are
//! native text, read here as blocks, so a note's words are marked in them. A page's words are inside its web view
//! (`reader_page.rs`), which reports the selection and where each block stands.
//!
//! Every verdict goes through `answer_items` with the notes as `{text, on: {quote, block}}`. The notes are kept as he
//! types, so a note half written is there when he comes back (`drafts.rs`).

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use gpui::{AppContext as _, Bounds, Context, Entity, Pixels, ScrollHandle, SharedString, Subscription, Window};
use gpui_component::input::{InputEvent, TextareaState};
use gpui_component::text::TextViewState;
use ipc::{AnswerItems, ArtifactForm, ItemAnswer, ItemHow, ItemNote, NoteOn, SessionId};
use serde::{Deserialize, Serialize};

use super::acts::{Answer, SHEET};
use super::band_card::Stand;
use super::cards::{settled_of, Said};
use super::row_tabs::RowKind;
use super::JobView;

/// The reader's width, as it decides what draws: the contents rail and the gutter from here up.
pub(super) const WIDE: f32 = 880.;
/// Below this the gutter has no room, and each note sits under its paragraph.
pub(super) const MEDIUM: f32 = 560.;

/// How much of the reader's width is left for it to draw in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Room {
    /// Contents, words and gutter.
    Wide,
    /// Words and gutter, the contents in a menu.
    Medium,
    /// Words, each note under its paragraph, the contents in a menu.
    Narrow,
}

impl Room {
    pub(super) fn of(width: f32) -> Self {
        if width >= WIDE {
            Room::Wide
        } else if width >= MEDIUM {
            Room::Medium
        } else {
            Room::Narrow
        }
    }

    pub(super) fn rail(self) -> bool {
        self == Room::Wide
    }

    pub(super) fn gutter(self) -> bool {
        self != Room::Narrow
    }
}

/// The width the notes' margin takes beside the words: none while notes are off, and none where there is no room for it.
pub(super) fn notes_margin(on: bool, room: Room, look: &crate::look::Look) -> Pixels {
    use crate::look::token;
    if on && room.gutter() {
        look.length(token::SPACE_12) * 4. + look.length(token::SPACE_1) + look.length(token::SPACE_6)
    } else {
        gpui::px(0.)
    }
}

// ---- the words, as blocks -------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BlockKind {
    /// A heading of this level (1 to 6): one line.
    Heading(u8),
    /// A table, which takes the whole width.
    Table,
    /// A paragraph, a list, a quote or code.
    Text,
}

/// One block of a draft's markdown: what a note points at and what moved is told by.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Block {
    /// `b0`, `b1`, in the order the blocks stand.
    pub id: String,
    pub kind: BlockKind,
    pub source: String,
}

fn heading_level(line: &str) -> Option<u8> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&hashes) && line[hashes..].starts_with(' ')).then_some(hashes as u8)
}

/// The words of a heading line, without its marks.
pub(super) fn heading_words(source: &str) -> String {
    source.trim_start_matches('#').trim().replace(['*', '`'], "")
}

/// A draft's markdown cut at its blank lines, a heading on a line of its own, and a fence kept whole.
pub(super) fn blocks_of(text: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut lines: Vec<&str> = Vec::new();
    let mut fenced = false;
    let mut flush = |lines: &mut Vec<&str>, out: &mut Vec<Block>| {
        if lines.is_empty() {
            return;
        }
        let source = lines.join("\n");
        let first = lines[0].trim_start();
        let kind = if lines.len() == 1 && heading_level(first).is_some() {
            BlockKind::Heading(heading_level(first).unwrap_or(1))
        } else if lines.len() >= 2 && lines.iter().all(|line| line.trim_start().starts_with('|')) {
            BlockKind::Table
        } else {
            BlockKind::Text
        };
        out.push(Block { id: format!("b{}", out.len()), kind, source });
        lines.clear();
    };
    let text = text.replace('\r', "");
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            lines.push(line);
            continue;
        }
        if fenced {
            lines.push(line);
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut lines, &mut out);
        } else if heading_level(trimmed).is_some() {
            flush(&mut lines, &mut out);
            lines.push(line);
            flush(&mut lines, &mut out);
        } else {
            lines.push(line);
        }
    }
    flush(&mut lines, &mut out);
    out
}

/// The blocks put back as the markdown they were cut from, with one blank line between.
pub(super) fn text_of(blocks: &[String]) -> String {
    blocks.iter().map(|block| block.trim_end()).filter(|block| !block.trim().is_empty()).collect::<Vec<_>>().join("\n\n")
}

/// Which blocks differ from the text they replace: a block moved where its words are not a block of the earlier text.
pub(super) fn moved_since(before: &str, now: &[Block]) -> Vec<bool> {
    let old: Vec<String> = blocks_of(before).into_iter().map(|block| squeezed(&block.source)).collect();
    now.iter().map(|block| !old.contains(&squeezed(&block.source))).collect()
}

fn squeezed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Where the words he marked stand in the text a view draws: as written, else with the white space as it was squeezed.
pub(super) fn locate(rendered: &str, quote: &str) -> Option<Range<usize>> {
    let quote = quote.trim();
    if quote.is_empty() {
        return None;
    }
    if let Some(at) = rendered.find(quote) {
        return Some(at..at + quote.len());
    }
    // The same words with their white space squeezed on both sides, mapped back to where they began.
    let mut squeezed_rendered = String::new();
    let mut starts: Vec<usize> = Vec::new();
    let mut space = true;
    for (at, c) in rendered.char_indices() {
        if c.is_whitespace() {
            if !space {
                squeezed_rendered.push(' ');
                starts.push(at);
            }
            space = true;
        } else {
            for _ in 0..c.len_utf8() {
                starts.push(at);
            }
            squeezed_rendered.push(c);
            space = false;
        }
    }
    let wanted = squeezed(quote);
    let at = squeezed_rendered.find(&wanted)?;
    let start = *starts.get(at)?;
    let end_at = at + wanted.len();
    let end = starts.get(end_at).copied().unwrap_or(rendered.len());
    Some(start..end.max(start))
}

/// The block whose words hold the quote: the first, so a quote that crosses two blocks sits on the one it starts in.
pub(super) fn block_holding(blocks: &[Block], quote: &str) -> Option<usize> {
    let wanted = squeezed(quote);
    if wanted.is_empty() {
        return None;
    }
    blocks.iter().position(|block| squeezed(&block.source).contains(&wanted)).or_else(|| {
        let first = wanted.split(' ').take(4).collect::<Vec<_>>().join(" ");
        blocks.iter().position(|block| squeezed(&block.source).contains(&first))
    })
}

// ---- notes -----------------------------------------------------------------------------------

/// One note as it is kept between runs: the words he marked, where, and what he wrote.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(super) struct Note {
    #[serde(default)]
    pub quote: String,
    #[serde(default)]
    pub block: String,
    #[serde(default)]
    pub text: String,
}

/// The notes as `answer_items` takes them: a note with no words is left out, a note with no quote has no `on`.
pub(super) fn items_of(notes: &[Note]) -> Vec<ItemNote> {
    notes
        .iter()
        .filter(|note| !note.text.trim().is_empty())
        .map(|note| ItemNote {
            text: note.text.trim().to_string(),
            on: (!note.quote.trim().is_empty()).then(|| NoteOn { quote: note.quote.trim().to_string(), block: note.block.clone() }),
        })
        .collect()
}

/// What a card reads of a verdict's notes: one line each.
pub(super) fn said_of(notes: &[ItemNote]) -> String {
    notes.iter().enumerate().map(|(at, note)| format!("{}. {}", at + 1, note.text)).collect::<Vec<_>>().join("\n")
}

/// `answer_items` for one item: his verdict and its notes.
pub(super) fn verdict(job: &str, item: &str, how: ItemHow, notes: Vec<ItemNote>) -> AnswerItems {
    AnswerItems { items: vec![ItemAnswer { session_id: SessionId::carried(job.to_string()), item: item.to_string(), how, notes, extras: Default::default() }] }
}

/// The key the reader keeps a thing's notes under (`drafts.rs`).
pub(super) fn notes_key(job: &str, artifact: &str) -> String {
    format!("reader-notes:{job}:{artifact}")
}

/// The key the reader keeps a draft's edited blocks under.
pub(super) fn edit_key(job: &str, artifact: &str) -> String {
    format!("reader-edit:{job}:{artifact}")
}

/// How the notes are put into one kept value, and read back.
pub(super) fn notes_kept(notes: &[Note]) -> String {
    if notes.iter().all(|note| note.text.is_empty() && note.quote.is_empty()) {
        return String::new();
    }
    ipc::encode(&notes).unwrap_or_default()
}

pub(super) fn notes_read(kept: &str) -> Vec<Note> {
    if kept.is_empty() {
        return Vec::new();
    }
    ipc::decode::<Vec<Note>>("kept notes", kept.as_bytes()).unwrap_or_default()
}

/// Where a block stands among its page's or draft's blocks: `b3` is the fourth.
pub(super) fn block_number(block: &str) -> Option<usize> {
    block.strip_prefix('b')?.parse().ok()
}

/// A note's place among the others: by the block it is on, then the order he made them in.
pub(super) fn place_of(blocks: &[Block], notes: &[Note], note: &Note) -> usize {
    let _ = notes;
    blocks.iter().position(|block| block.id == note.block).unwrap_or(usize::MAX)
}

// ---- what the reader holds -----------------------------------------------------------------

/// A note, drawn in the gutter with its words in a box that keeps what is typed in it.
pub(super) struct NoteBox {
    pub quote: String,
    pub block: String,
    pub area: Entity<TextareaState>,
    _typing: Subscription,
}

/// A block of a draft, with the view its words are drawn through where it has any, and where it stood last frame.
pub(super) struct BlockView {
    pub block: Block,
    pub state: Option<Entity<TextViewState>>,
    pub bounds: Rc<Cell<Bounds<Pixels>>>,
    /// What the marks were last painted for: the rendered words and the quotes, so a repaint of the same is skipped.
    pub stamp: Option<(String, Vec<String>)>,
    _read: Option<Subscription>,
}

/// Edit on: a box for each block of a draft, and the blocks as they were; on a page, the page he types in.
pub(super) struct Edit {
    pub areas: Vec<Entity<TextareaState>>,
    pub original: Vec<String>,
    _typing: Vec<Subscription>,
    pub page: Option<super::reader_page_edit::PageEdit>,
}

impl Edit {
    pub(super) fn new(areas: Vec<Entity<TextareaState>>, original: Vec<String>, typing: Vec<Subscription>) -> Self {
        Edit { areas, original, _typing: typing, page: None }
    }

    pub(super) fn page(page: super::reader_page_edit::PageEdit) -> Self {
        Edit { areas: Vec::new(), original: Vec::new(), _typing: Vec::new(), page: Some(page) }
    }

    pub(super) fn words(&self, cx: &gpui::App) -> Vec<String> {
        self.areas.iter().map(|area| area.read(cx).value().to_string()).collect()
    }

    /// Which blocks he has changed.
    pub(super) fn changed(&self, cx: &gpui::App) -> Vec<bool> {
        self.areas.iter().zip(&self.original).map(|(area, was)| squeezed(area.read(cx).value().as_ref()) != squeezed(was)).collect()
    }
}

/// Everything one open reader remembers.
pub(super) struct Reader {
    pub job: SharedString,
    pub kind: RowKind,
    pub key: String,
    /// A draft's text as the blocks were made from it, and the text it replaced.
    pub text: String,
    pub before: Option<String>,
    pub blocks: Vec<BlockView>,
    /// Which blocks moved since the revision he last opened.
    pub moved: Vec<bool>,
    pub notes: Vec<NoteBox>,
    /// N turns the notes' margin on: while it is off the page takes the whole width.
    pub notes_on: bool,
    /// The note J and K are on.
    pub on: Option<usize>,
    /// The revision he chose to read, or the newest.
    pub viewing: Option<u32>,
    /// The newest revision he had opened before this one, which what moved is told from.
    pub base: Option<u32>,
    /// The newest revision marked seen, so it is marked once.
    pub marked: Option<u32>,
    /// Every revision Fleet was told he opened from here, so one is told once however often it draws.
    pub told: std::collections::BTreeSet<u32>,
    pub edit: Option<Edit>,
    pub saving: bool,
    pub scroll: ScrollHandle,
    pub width: Rc<Cell<Pixels>>,
    /// The contents entry the words are at, for a page set by the page and for a draft by the scroll.
    pub said: Option<SharedString>,
    /// The heading of a page the contents mark: the one last gone to, since the page scrolls inside its own view.
    pub at_head: usize,
    /// What a save from Share came to, said in the band for a moment.
    pub shared: Option<SharedString>,
    /// Ask for changes, opened in place under the answers: what should change, sent with the notes on the words.
    pub ask: Option<(Entity<TextareaState>, Subscription)>,
    /// A page's margin: his comments on its passages and the agent's answers, as Fleet keeps them.
    pub margin: super::margin::Margin,
}

impl Reader {
    pub(super) fn new(job: SharedString, kind: RowKind, key: String) -> Self {
        Reader {
            job,
            kind,
            key,
            text: String::new(),
            before: None,
            blocks: Vec::new(),
            moved: Vec::new(),
            notes: Vec::new(),
            notes_on: false,
            on: None,
            viewing: None,
            base: None,
            marked: None,
            told: std::collections::BTreeSet::new(),
            edit: None,
            saving: false,
            scroll: ScrollHandle::new(),
            width: Rc::new(Cell::new(gpui::px(0.))),
            said: None,
            at_head: 0,
            shared: None,
            ask: None,
            margin: super::margin::Margin::default(),
        }
    }

    pub(super) fn id(&self) -> String {
        reader_id(self.kind, &self.key)
    }

    pub(super) fn room(&self) -> Room {
        Room::of(f32::from(self.width.get()))
    }

    pub(super) fn heads(&self) -> Vec<(usize, u8, String)> {
        self.blocks.iter().enumerate().filter_map(|(at, one)| match one.block.kind {
            BlockKind::Heading(level) => Some((at, level, heading_words(&one.block.source))),
            _ => None,
        }).collect()
    }

    /// Whether the caret is in a note or a block he edits, where a letter is a letter.
    pub(super) fn typing(&self, window: &Window, cx: &gpui::App) -> bool {
        use gpui::Focusable as _;
        let in_area = |area: &Entity<TextareaState>| area.read(cx).focus_handle(cx).contains_focused(window, cx);
        self.notes.iter().any(|one| in_area(&one.area))
            || self.edit.as_ref().is_some_and(|edit| edit.areas.iter().any(in_area))
            || self.ask.as_ref().is_some_and(|(area, _)| in_area(area))
            || self.margin.typing(window, cx)
    }

    /// The notes as `Note`s, as they are kept.
    pub(super) fn kept_notes(&self, cx: &gpui::App) -> Vec<Note> {
        self.notes.iter().map(|one| Note { quote: one.quote.clone(), block: one.block.clone(), text: one.area.read(cx).value().to_string() }).collect()
    }
}

/// One reader's name in the Job's table of them.
pub(super) fn reader_id(kind: RowKind, key: &str) -> String {
    format!("{}:{key}", super::panes_place::row_kind_name(kind))
}

impl JobView {
    pub(super) fn reader_of(&self, id: &str, reader: &str) -> Option<&Reader> {
        self.row_tabs.get(id)?.readers.get(reader)
    }

    pub(super) fn reader_mut(&mut self, id: &str, reader: &str) -> Option<&mut Reader> {
        self.row_tabs.get_mut(id)?.readers.get_mut(reader)
    }

    /// A box for a note's words, which keeps them as they are typed.
    fn note_area(&mut self, reader: &str, text: &str, window: &mut Window, cx: &mut Context<Self>) -> (Entity<TextareaState>, Subscription) {
        let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 8));
        area.update(cx, |area, cx| area.set_value(text.to_string(), window, cx));
        let reader = reader.to_string();
        let typing = cx.subscribe_in(&area, window, move |this, _, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                this.reader_notes_changed(&reader, cx);
            }
        });
        (area, typing)
    }

    /// His notes were typed in: kept, so they are there when he comes back.
    fn reader_notes_changed(&mut self, reader: &str, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(held) = self.reader_of(&id, reader) else { return };
        let (artifact, notes) = (self.reader_artifact(&id, reader, cx), held.kept_notes(cx));
        if let Some(artifact) = artifact {
            self.keep_words(&notes_key(&id, &artifact), &notes_kept(&notes), cx);
        }
        self.reader_paint(&id, reader, cx);
        cx.notify();
    }

    /// The artifact a reader reads, by its `JobArtifact` id.
    pub(super) fn reader_artifact(&self, id: &str, reader: &str, cx: &Context<Self>) -> Option<String> {
        let held = self.reader_of(id, reader)?;
        match held.kind {
            RowKind::Document => Some(held.key.clone()),
            RowKind::Draft => {
                let items = &self.thread(id, cx)?.items;
                match super::review::draft_row(items, &held.key)? {
                    ipc::SessionRow::Draft { title, .. } => Some(format!("draft:{title}")),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// A note on the words picked, or on the block where nothing is: it is made, and the caret is in it.
    pub(super) fn reader_add_note(&mut self, id: &SharedString, reader: &str, quote: String, block: String, window: &mut Window, cx: &mut Context<Self>) {
        let (area, typing) = self.note_area(reader, "", window, cx);
        let Some(held) = self.reader_mut(id, reader) else { return };
        let at = block_number(&block);
        // After the notes already on its block, before those on a later one.
        let place = |one: &NoteBox| block_number(&one.block).unwrap_or(usize::MAX);
        let index = held.notes.iter().position(|one| at.is_some_and(|at| place(one) > at)).unwrap_or(held.notes.len());
        held.notes.insert(index, NoteBox { quote, block, area: area.clone(), _typing: typing });
        held.notes_on = true;
        held.on = Some(index);
        if let Some(at) = at.filter(|_| held.kind == RowKind::Draft) {
            held.scroll.scroll_to_item(at);
        }
        area.update(cx, |area, cx| area.focus(window, cx));
        self.reader_notes_changed(reader, cx);
    }

    /// N: a note on the words he picked.
    pub(super) fn reader_note_key(&mut self, id: &SharedString, reader: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, reader) else { return };
        if held.edit.is_some() {
            return;
        }
        // A page's words are in its web view, which says what he selected.
        if held.kind == RowKind::Document {
            self.reader_page_note(id, reader, true, window, cx);
            return;
        }
        // The block that holds a selection says so itself.
        let picked = held.blocks.iter().find_map(|one| {
            let words = one.state.as_ref().map(|state| state.read(cx).selected_text()).unwrap_or_default();
            (!words.trim().is_empty()).then(|| (words, one.block.id.clone()))
        });
        // With nothing picked N is the switch of the margin.
        let Some((quote, block)) = picked else {
            self.reader_toggle_notes(id, reader, cx);
            return;
        };
        let states: Vec<_> = self.reader_of(id, reader).map(|held| held.blocks.iter().filter_map(|one| one.state.clone()).collect()).unwrap_or_default();
        for state in states {
            state.update(cx, |state, cx| state.clear_selection(cx));
        }
        gpui_base::TextSelection::clear(window, cx);
        self.reader_add_note(id, reader, quote.trim().to_string(), block, window, cx);
    }

    /// The notes' margin on or off: off, the words take the whole width.
    pub(super) fn reader_toggle_notes(&mut self, id: &SharedString, reader: &str, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, reader) {
            held.notes_on = !held.notes_on;
            cx.notify();
        }
    }

    /// J and K: the next note or the one before, brought into view.
    pub(super) fn reader_move(&mut self, id: &SharedString, reader: &str, down: bool, cx: &mut Context<Self>) {
        // On a page J and K go down the margin's threads.
        if self.reader_of(id, reader).is_some_and(|held| held.kind == RowKind::Document && held.margin.open().next().is_some()) {
            self.margin_move(id, reader, down, cx);
            return;
        }
        let Some(held) = self.reader_mut(id, reader) else { return };
        if held.notes.is_empty() {
            return;
        }
        let last = held.notes.len() - 1;
        let at = match (held.on, down) {
            (None, true) => 0,
            (None, false) => last,
            (Some(at), true) => (at + 1).min(last),
            (Some(at), false) => at.saturating_sub(1),
        };
        if held.on != Some(at) {
            crate::ui_sounds::moved();
        }
        held.on = Some(at);
        let block = held.blocks.iter().position(|one| one.block.id == held.notes[at].block);
        if let Some(block) = block {
            held.scroll.scroll_to_item(block);
        }
        self.reader_show_note(id, reader, at, cx);
        cx.notify();
    }

    /// A note's words shown where they stand: a draft scrolls to their block, a page selects them.
    pub(super) fn reader_show_note(&mut self, id: &SharedString, reader: &str, place: usize, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, reader) else { return };
        held.on = Some(place);
        if held.kind == RowKind::Document {
            self.reader_page_show(id, reader, place, cx);
            return;
        }
        let block = held.notes.get(place).and_then(|note| held.blocks.iter().position(|one| one.block.id == note.block));
        if let Some(block) = block {
            held.scroll.scroll_to_item(block);
        }
        cx.notify();
    }

    /// Enter on the note J and K are on: the caret goes into it.
    pub(super) fn reader_into_note(&mut self, id: &SharedString, reader: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.reader_of(id, reader).is_some_and(|held| held.kind == RowKind::Document && held.margin.on.is_some()) {
            self.margin_into(id, reader, window, cx);
            return;
        }
        let area = self.reader_of(id, reader).and_then(|held| held.on.and_then(|at| held.notes.get(at))).map(|one| one.area.clone());
        if let Some(area) = area {
            area.update(cx, |area, cx| area.focus(window, cx));
        }
    }

    /// Backspace on the note J and K are on, or Esc in a note with nothing in it: it goes.
    pub(super) fn reader_drop_note(&mut self, id: &SharedString, reader: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, reader) else { return };
        let Some(at) = held.on.filter(|at| *at < held.notes.len()) else { return };
        held.notes.remove(at);
        held.on = if held.notes.is_empty() { None } else { Some(at.min(held.notes.len() - 1)) };
        window.focus(&self.row_focus, cx);
        self.reader_notes_changed(reader, cx);
    }

    /// Ask for changes opened in place, the caret in its box.
    pub(super) fn reader_ask_open(&mut self, id: &SharedString, reader: &str, window: &mut Window, cx: &mut Context<Self>) {
        let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(2, 8).placeholder("What should change?"));
        let typing = cx.subscribe_in(&area, window, |_, _, event: &InputEvent, _, cx| {
            if let InputEvent::Change = event {
                cx.notify();
            }
        });
        let Some(held) = self.reader_mut(id, reader) else { return };
        held.ask = Some((area.clone(), typing));
        area.update(cx, |area, cx| area.focus(window, cx));
        cx.notify();
    }

    /// Cancel or Esc: the box goes with what was typed in it, and the notes on the words stay.
    pub(super) fn reader_ask_cancel(&mut self, id: &SharedString, reader: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, reader) {
            held.ask = None;
        }
        window.focus(&self.row_focus, cx);
        cx.notify();
    }

    /// The notes were sent with a verdict: they and what was kept for them go.
    fn reader_notes_spent(&mut self, id: &SharedString, reader: &str, cx: &mut Context<Self>) {
        let artifact = self.reader_artifact(id, reader, cx);
        if let Some(held) = self.reader_mut(id, reader) {
            held.notes.clear();
            held.on = None;
            held.ask = None;
        }
        if let Some(artifact) = artifact {
            self.keep_words(&notes_key(id, &artifact), "", cx);
        }
        self.reader_paint(id, reader, cx);
    }

    /// His verdict, with his notes, as `answer_items` takes it. The card reads it at once; Fleet's refusal takes it back.
    pub(super) fn reader_answer(&mut self, id: &SharedString, reader: &str, how: ItemHow, card_key: String, item: String, window: &mut Window, cx: &mut Context<Self>) {
        let mut notes = self.reader_of(id, reader).map(|held| items_of(&held.kept_notes(cx))).unwrap_or_default();
        if how == ItemHow::SendBack {
            // Ask for changes opens a box in place first; its words go with the notes on the words, as a note on the whole.
            let typed = self.reader_of(id, reader).and_then(|held| held.ask.as_ref()).map(|(area, _)| (area.clone(), area.read(cx).value().trim().to_string()));
            match typed {
                None => {
                    self.reader_ask_open(id, reader, window, cx);
                    return;
                }
                Some((area, words)) if words.is_empty() && notes.is_empty() => {
                    area.update(cx, |area, cx| area.focus(window, cx));
                    return;
                }
                Some((_, words)) if !words.is_empty() => notes.push(ItemNote { text: words, on: None }),
                Some(_) => {}
            }
        }
        let answer = match how {
            ItemHow::Approve => Answer::Approved,
            ItemHow::SendBack => Answer::SentBack(said_of(&notes)),
        };
        let body = ipc::encode(&verdict(id, &item, how, notes));
        self.reader_notes_spent(id, reader, cx);
        if let Some(cards) = self.cards_mut() {
            cards.answers.insert(card_key.clone(), answer);
            cards.said.remove(&card_key);
        }
        self.card_changed(SHEET, cx);
        let (job, key) = (id.clone(), card_key);
        let landed = move |this: &mut JobView, answer: Result<Vec<u8>, String>, _window: &mut Window, cx: &mut Context<JobView>| {
            let refused = answer.and_then(|bytes| ipc::decode::<ipc::ItemsSettled>("ItemsSettled", &bytes).map_err(|why| why.to_string())).map(|settled| {
                settled.outcomes.into_iter().find_map(|one| match one.outcome {
                    ipc::ItemSettled::Refused { message, .. } => Some(message),
                    _ => None,
                })
            });
            match refused {
                Ok(None) => {}
                Ok(Some(why)) | Err(why) => this.undo(&job, SHEET, &key, why.into(), cx),
            }
        };
        if self.fleet.is_none() {
            let settled = body.map_err(|why| format!("{why:?}")).and_then(|body| crate::mock::items_settled("/sessions/items/answer", &body));
            let bytes = settled.and_then(|settled| ipc::encode(&settled).map(String::into_bytes).map_err(|why| format!("{why:?}")));
            landed(self, bytes, window, cx);
            return;
        }
        self.post("/sessions/items/answer".to_string(), body, window, cx, Box::new(landed));
    }
}

/// What a reader shows of the thing it reads, worked out from the thread each frame.
pub(super) struct Subject {
    pub form: ArtifactForm,
    pub title: String,
    /// The `JobArtifact` id.
    pub artifact: String,
    /// The waiting item that answers it: `document:<page>` or `draft:<row>`.
    pub item: String,
    /// What its card is kept under, where its answer is held before Fleet's row says it.
    pub card_key: String,
    pub stand: Stand,
    pub words: Option<String>,
    pub for_line: Option<ipc::ArtifactFor>,
    /// Put to him and not answered, so Approve and Ask for changes show.
    pub asking: bool,
    /// Answered or put away, so Edit is refused.
    pub closed: bool,
    pub archived: bool,
    /// The newest revision, and every one kept with who made it.
    pub newest: u32,
    pub revisions: Vec<ipc::ArtifactRevision>,
    /// The newest revision he has opened.
    pub seen: Option<u32>,
    /// A draft's words and the words they replace; a page's path.
    pub text: Option<String>,
    pub before: Option<String>,
    pub page: Option<String>,
    /// A draft's row.
    pub row: Option<String>,
}

impl JobView {
    /// What the thread says of the draft or page a reader reads. `None` where its row or card has gone.
    pub(super) fn subject_of(&self, id: &SharedString, kind: RowKind, key: &str, cx: &Context<Self>) -> Option<Subject> {
        let held = self.thread(id, cx)?;
        let page = self.pages.get(id)?;
        match kind {
            RowKind::Document => {
                let card = super::review::document_card(&held.items, key)?;
                let id_key = card.id.clone();
                let settled = settled_of(card.settled.as_ref(), page.cards.answers.get(&id_key));
                let kept = self.artifact_of(id, &card.page);
                let archived = kept.is_some_and(|one| one.archived.is_some());
                let dismissed = page.cards.answers.get(&id_key) == Some(&Answer::Withdrawn)
                    || kept.is_some_and(|one| matches!(one.state, ipc::ArtifactState::Dismissed | ipc::ArtifactState::Withdrawn) && settled.is_none());
                let stand = match (&settled, dismissed) {
                    (Some((Said::Approved, _)), _) => Stand::Approved,
                    (Some((Said::SentBack, _)), _) => Stand::SentBack,
                    (None, true) => kept.filter(|one| one.state == ipc::ArtifactState::Withdrawn).map_or(Stand::Dismissed, |_| Stand::Withdrawn),
                    (None, false) if card.asked => Stand::Asking,
                    (None, false) => Stand::Reading,
                };
                Some(Subject {
                    form: ArtifactForm::Page,
                    title: card.title.clone(),
                    artifact: card.page.clone(),
                    item: format!("document:{}", card.page),
                    card_key: id_key,
                    stand,
                    words: kept.and_then(|one| one.words.clone()),
                    for_line: card.for_line.clone(),
                    asking: stand == Stand::Asking && !archived,
                    closed: !matches!(stand, Stand::Asking | Stand::Reading) || archived,
                    archived,
                    newest: kept.and_then(|one| one.revisions.last()).map_or(card.revision, |last| last.number.max(card.revision)),
                    revisions: kept.map(|one| one.revisions.clone()).unwrap_or_default(),
                    seen: kept.and_then(|one| one.seen),
                    text: None,
                    before: None,
                    page: Some(card.page.clone()),
                    row: None,
                })
            }
            RowKind::Draft => {
                let ipc::SessionRow::Draft { id: row, title, text, before, settled, change: None, for_line, .. } = super::review::draft_row(&held.items, key)? else { return None };
                let card_key = format!("draft:{row}");
                let how = match page.cards.answers.get(&card_key) {
                    Some(Answer::Approved) => Some(ipc::DraftHow::Approved),
                    Some(Answer::Fixed(_)) => Some(ipc::DraftHow::Fixed),
                    Some(Answer::SentBack(_)) => Some(ipc::DraftHow::SentBack),
                    Some(Answer::Withdrawn | Answer::Dismissed) => Some(ipc::DraftHow::Withdrawn),
                    _ => settled.as_ref().map(|settled| settled.how),
                };
                let stand = match how {
                    None => Stand::Asking,
                    Some(ipc::DraftHow::Approved | ipc::DraftHow::Fixed) => Stand::Approved,
                    Some(ipc::DraftHow::SentBack) => Stand::SentBack,
                    Some(ipc::DraftHow::Withdrawn) => Stand::Dismissed,
                };
                let artifact = format!("draft:{title}");
                let kept = self.artifact_of(id, &artifact);
                let archived = kept.is_some_and(|one| one.archived.is_some());
                let revisions = kept.map(|one| one.revisions.clone()).unwrap_or_default();
                Some(Subject {
                    form: ArtifactForm::Draft,
                    title: title.clone(),
                    artifact,
                    item: card_key.clone(),
                    card_key,
                    stand,
                    words: kept.and_then(|one| one.words.clone()),
                    for_line: for_line.clone(),
                    asking: stand == Stand::Asking && !archived,
                    closed: stand != Stand::Asking || archived,
                    archived,
                    newest: revisions.last().map_or(1, |last| last.number),
                    revisions,
                    seen: kept.and_then(|one| one.seen),
                    text: Some(text.clone()),
                    before: before.clone(),
                    page: None,
                    row: Some(row.clone()),
                })
            }
            _ => None,
        }
    }

    /// Each draft and page tab holds its reader once, read from the thread as it comes, and a tab that closed lets its
    /// reader go. A draft's words are cut into blocks again where they changed; where they did, what moved is told.
    pub(super) fn sync_readers(&mut self, id: &SharedString, window: &mut Window, cx: &mut Context<Self>) {
        let tabs: Vec<_> = self.row_tab_list(id).into_iter().filter(|tab| matches!(tab.kind, RowKind::Draft | RowKind::Document)).collect();
        let wanted: Vec<String> = tabs.iter().map(|tab| reader_id(tab.kind, &tab.key)).collect();
        let held = self.row_tabs.entry(id.clone()).or_default();
        held.readers.retain(|name, _| wanted.contains(name));
        for tab in tabs {
            let name = reader_id(tab.kind, &tab.key);
            let Some(subject) = self.subject_of(id, tab.kind, &tab.key, cx) else { continue };
            if self.reader_of(id, &name).is_none() {
                let mut fresh = Reader::new(id.clone(), tab.kind, tab.key.clone());
                fresh.base = subject.seen.filter(|seen| *seen < subject.newest);
                // A page opens with its margin beside it.
                fresh.notes_on = tab.kind == RowKind::Document;
                self.row_tabs.entry(id.clone()).or_default().readers.insert(name.clone(), fresh);
                // The notes he left half written come back, each with the caret out of them.
                for note in notes_read(&self.draft_words(&notes_key(id, &subject.artifact))) {
                    let (area, typing) = self.note_area(&name, &note.text, window, cx);
                    if let Some(held) = self.reader_mut(id, &name) {
                        held.notes.push(NoteBox { quote: note.quote, block: note.block, area, _typing: typing });
                        // Notes he wrote stay in sight, so none is sent unseen.
                        held.notes_on = true;
                    }
                }
            }
            if let Some(newest) = subject.text.as_deref() {
                let earlier = self.reader_of(id, &name).is_some_and(|held| held.viewing.is_some_and(|at| at != subject.newest));
                let text = if earlier { subject.before.as_deref().unwrap_or(newest) } else { newest };
                let (stale, editing) = self.reader_of(id, &name).map_or((false, false), |held| (held.text != text || held.blocks.is_empty() && !text.is_empty(), held.edit.is_some()));
                if stale && !editing {
                    self.reader_blocks(id, &name, text, subject.before.clone(), window, cx);
                    if earlier {
                        if let Some(held) = self.reader_mut(id, &name) {
                            held.moved = vec![false; held.blocks.len()];
                        }
                    }
                }
            }
            self.reader_seen(id, &name, &subject, window, cx);
            self.margin_sync(id, &name, window, cx);
        }
    }

    /// A draft's words cut into blocks, each drawn through a view of its own so the words he marks can be painted.
    fn reader_blocks(&mut self, id: &SharedString, name: &str, text: &str, before: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let blocks = blocks_of(text);
        let mut views = Vec::new();
        for block in blocks {
            let (state, read) = match block.kind {
                BlockKind::Heading(_) => (None, None),
                _ => {
                    let state = cx.new(|cx| TextViewState::markdown(&block.source, cx));
                    let (wanted, id) = (name.to_string(), id.clone());
                    let read = cx.observe(&state, move |this, _, cx| this.reader_paint(&id, &wanted, cx));
                    (Some(state), Some(read))
                }
            };
            views.push(BlockView { block, state, bounds: Rc::new(Cell::new(Bounds::default())), stamp: None, _read: read });
        }
        let _ = window;
        let Some(held) = self.reader_mut(id, name) else { return };
        held.moved = held.base.and(before.as_deref()).map_or_else(|| vec![false; views.len()], |before| moved_since(before, &views.iter().map(|one| one.block.clone()).collect::<Vec<_>>()));
        held.text = text.to_string();
        held.before = before;
        held.blocks = views;
        cx.notify();
    }

    /// Paints the words each note marks, in his colour, where the view has read them.
    pub(super) fn reader_paint(&mut self, id: &SharedString, name: &str, cx: &mut Context<Self>) {
        use gpui_component::text::RangeHighlight;
        let mark = {
            use crate::look::ActiveLook;
            cx.look().color(crate::look::token::YOU).opacity(0.28)
        };
        let Some(held) = self.reader_of(id, name) else { return };
        let notes: Vec<(String, String)> = held.notes.iter().map(|one| (one.block.clone(), one.quote.clone())).collect();
        let current = held.on.and_then(|at| held.notes.get(at)).map(|one| (one.block.clone(), one.quote.clone()));
        let work: Vec<(usize, Entity<TextViewState>, Vec<String>)> = held
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(at, one)| one.state.clone().map(|state| (at, state, notes.iter().filter(|(block, quote)| *block == one.block.id && !quote.trim().is_empty()).map(|(_, quote)| quote.clone()).collect::<Vec<_>>())))
            .collect();
        for (at, state, quotes) in work {
            let rendered = state.read(cx).rendered_text();
            let text = rendered.as_str().to_string();
            let stamp = (text.clone(), quotes.clone());
            if self.reader_of(id, name).and_then(|held| held.blocks.get(at)).is_some_and(|one| one.stamp.as_ref() == Some(&stamp)) {
                continue;
            }
            let marks: Vec<RangeHighlight> = quotes.iter().filter_map(|quote| locate(&text, quote)).map(|range| RangeHighlight::new(range, mark)).collect();
            if !text.is_empty() {
                state.update(cx, |state, cx| {
                    state.set_range_highlights(marks, cx).ok();
                });
                if let Some(one) = self.reader_mut(id, name).and_then(|held| held.blocks.get_mut(at)) {
                    one.stamp = Some(stamp);
                }
            }
        }
        let _ = current;
    }

    /// He opened a revision: Fleet is told which, once for each, so it closes the Read item where it was the newest and decides
    /// from the number where it was an older one. A refusal is logged and changes nothing on screen.
    fn reader_seen(&mut self, id: &SharedString, name: &str, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        if subject.revisions.is_empty() {
            return;
        }
        let older = held.viewing.filter(|at| *at != subject.newest);
        let shown = older.unwrap_or(subject.newest);
        if older.is_some() {
            if !held.told.insert(shown) {
                return;
            }
        } else {
            if held.marked == Some(subject.newest) {
                return;
            }
            // A newer revision arrived while it was open: it moved from the one he had in front of him.
            if let Some(before) = held.marked.filter(|marked| *marked < subject.newest) {
                held.base = Some(before);
                if held.kind == RowKind::Draft {
                    held.text.clear();
                    held.blocks.clear();
                }
            }
            held.marked = Some(subject.newest);
            if !held.told.insert(shown) || subject.seen.is_some_and(|seen| seen >= shown) {
                return;
            }
        }
        let body = ipc::encode(&ipc::MarkArtifactSeen { session_id: SessionId::carried(id.to_string()), artifact: subject.artifact.clone(), revision: shown });
        if self.fleet.is_none() {
            self.artifacts_seen_post(id, &subject.artifact, shown);
            return;
        }
        let artifact = subject.artifact.clone();
        self.post(
            "/sessions/artifacts/seen".to_string(),
            body,
            window,
            cx,
            Box::new(move |_, answer, _, _| {
                if let Err(why) = answer {
                    crate::log::Note::warn("Fleet did not take that he opened a revision").field("artifact", artifact).field("revision", shown).field("why", why).emit();
                }
            }),
        );
    }
}

/// What the reader reads of a thing: a draft is native words, a page is a web view.
pub(super) fn is_page(form: ArtifactForm) -> bool {
    form == ArtifactForm::Page
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# Workflows in Jobs\n\nEach Job draws its workflow as a bar of steps under the title.\n\n## The step bar\n\n- one\n- two\n\n| Step | State |\n|---|---|\n| Plan | Approved |\n\n```sh\nfoo\n\nbar\n```\n\nLast words.";

    #[test]
    fn markdown_is_cut_at_blank_lines_and_headings_stand_alone() {
        let blocks = blocks_of(TEXT);
        let kinds: Vec<_> = blocks.iter().map(|one| one.kind).collect();
        assert_eq!(
            kinds,
            [BlockKind::Heading(1), BlockKind::Text, BlockKind::Heading(2), BlockKind::Text, BlockKind::Table, BlockKind::Text, BlockKind::Text],
            "{blocks:?}"
        );
        assert_eq!(blocks[0].id, "b0");
        assert_eq!(blocks[5].source, "```sh\nfoo\n\nbar\n```", "a fence is one block, blank line and all");
        assert_eq!(heading_words(&blocks[2].source), "The step bar");
        let two = blocks_of("Intro\n## Under\nbody");
        assert_eq!(two.iter().map(|one| one.source.as_str()).collect::<Vec<_>>(), ["Intro", "## Under", "body"], "a heading with no blank line round it is still a block of its own");
    }

    #[test]
    fn the_blocks_put_back_read_as_the_text_with_one_blank_line_between() {
        let blocks: Vec<String> = blocks_of(TEXT).into_iter().map(|one| one.source).collect();
        assert_eq!(blocks_of(&text_of(&blocks)), blocks_of(TEXT));
        assert_eq!(text_of(&["a".into(), "".into(), "b ".into()]), "a\n\nb");
    }

    #[test]
    fn a_block_moved_when_its_words_are_not_a_block_of_the_earlier_text() {
        let now = blocks_of("Kept words.\n\nNew words here.\n\n## Same head");
        assert_eq!(moved_since("Kept   words.\n\n## Same head\n\nGone words.", &now), [false, true, false], "white space does not move a block");
    }

    #[test]
    fn words_are_found_where_the_view_draws_them_even_when_a_line_broke_between() {
        assert_eq!(locate("a bar of steps under the title", "steps under"), Some(9..20));
        let range = locate("a bar of steps\nunder the title", "steps under").expect("found across the break");
        assert_eq!(&"a bar of steps\nunder the title"[range], "steps\nunder");
        assert_eq!(locate("nothing here", "steps"), None);
        assert_eq!(locate("anything", "   "), None);
    }

    #[test]
    fn a_quote_belongs_to_the_block_it_starts_in() {
        let blocks = blocks_of(TEXT);
        assert_eq!(block_holding(&blocks, "steps under the title"), Some(1));
        assert_eq!(block_holding(&blocks, "Last words."), Some(6));
        assert_eq!(block_holding(&blocks, "Each Job draws its workflow as a bar of steps under the title.\n\nThe step"), Some(1), "the first words say where it began");
        assert_eq!(block_holding(&blocks, "never said"), None);
    }

    #[test]
    fn notes_go_with_the_verdict_as_the_wire_names_them() {
        let notes = vec![
            Note { quote: "steps under the title".into(), block: "b1".into(), text: " Beside the title instead. ".into() },
            Note { quote: String::new(), block: "b1".into(), text: "Drop the Ran column.".into() },
            Note { quote: "empty".into(), block: "b2".into(), text: "  ".into() },
        ];
        let items = items_of(&notes);
        assert_eq!(items.len(), 2, "a note with no words is left out");
        assert_eq!(items[0], ItemNote { text: "Beside the title instead.".into(), on: Some(NoteOn { quote: "steps under the title".into(), block: "b1".into() }) });
        assert_eq!(items[1].on, None, "a note with no quote has no `on`");
        assert_eq!(said_of(&items), "1. Beside the title instead.\n2. Drop the Ran column.");
        let body = verdict("mock-01", "draft:dra-1", ItemHow::SendBack, items);
        let wire = ipc::encode(&body).unwrap();
        assert!(wire.contains(r#""how":"send_back""#) && wire.contains(r#""on":{"quote":"steps under the title","block":"b1"}"#), "{wire}");
    }

    #[test]
    fn a_block_is_numbered_by_its_name() {
        assert_eq!((block_number("b0"), block_number("b12"), block_number("x"), block_number("")), (Some(0), Some(12), None, None));
    }

    #[test]
    fn notes_are_kept_and_read_back_and_none_keeps_nothing() {
        let notes = vec![Note { quote: "a".into(), block: "b0".into(), text: "b".into() }];
        assert_eq!(notes_read(&notes_kept(&notes)), notes);
        assert_eq!(notes_kept(&[Note { quote: String::new(), block: "b0".into(), text: String::new() }]), "");
        assert!(notes_read("not json").is_empty());
        assert!(notes_read("").is_empty());
    }

    #[test]
    fn the_room_decides_what_draws() {
        assert_eq!(Room::of(1200.), Room::Wide);
        assert_eq!(Room::of(700.), Room::Medium);
        assert_eq!(Room::of(400.), Room::Narrow);
        assert!(Room::Wide.rail() && Room::Wide.gutter());
        assert!(!Room::Medium.rail() && Room::Medium.gutter());
        assert!(!Room::Narrow.gutter());
    }
}

impl JobView {
    /// A revision chosen from the menu: a draft draws the words it replaced, a page is read again from Fleet.
    pub(super) fn reader_view(&mut self, id: &SharedString, name: &str, number: u32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        let (kind, key) = (held.kind, held.key.clone());
        let Some(subject) = self.subject_of(id, kind, &key, cx) else { return };
        let Some(held) = self.reader_mut(id, name) else { return };
        held.viewing = (number != subject.newest).then_some(number);
        held.on = None;
        // A draft's words are drawn again from the revision chosen, by the next frame's sync.
        if kind == RowKind::Draft {
            held.text.clear();
        }
        if kind == RowKind::Document {
            // Edit on the page ends with what he typed kept, as the page draws the revision chosen.
            if self.reader_of(id, name).is_some_and(|held| held.edit.is_some()) {
                self.page_edit_end(id, name, true, false, window, cx);
            }
            self.reader_page_revision(id, name, number, window, cx);
        }
        cx.notify();
    }

    /// A heading of the contents: the words scroll to it.
    pub(super) fn reader_go(&mut self, id: &SharedString, name: &str, head: usize, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        if held.kind == RowKind::Document {
            self.reader_page_go(id, name, head, cx);
            return;
        }
        let Some((block, _, _)) = held.heads().get(head).cloned() else { return };
        held.scroll.scroll_to_top_of_item(block);
        cx.notify();
    }
}
