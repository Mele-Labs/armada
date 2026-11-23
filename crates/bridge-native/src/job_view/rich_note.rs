//! The box a person writes back to an agent in: Ask for changes on a card, a page part or a document, and Answer in words
//! on a question. It is the Job's message box in small, not a copy of it: pictures pasted or files dropped become a tray of
//! tiles, `@` offers Jobs and files and a Job picked is a chip, and the words are drawn and sent as a message's are
//! (`format.rs`).
//!
//! What it holds beyond the words is read back as the answer's `extras`: the files as uploads, the chips as mentions. A box
//! that sends an answer asks `sent` for the words and `extras` for the rest before the answer's card lets go of it.
//!
//! The text area is the host's own, made as it always was, so what a host does with its words (keeping them, Enter to send)
//! stays where it was. The note wraps it. What a Job's box has and this has not: `/` and `#` lists, the quick bar, Talk,
//! sketches and the pickers, which belong to a running Job's message.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{div, prelude::*, App, ClickEvent, ClipboardItem, Context, Entity, EventEmitter, ExternalPaths, SharedString, Subscription, Task, Window};
use gpui_component::input::{Enter, Escape, IndentInline, InputEvent, MoveDown, MoveUp, TextareaState};
use ipc::AnswerExtras;

use super::composer::{attached, clipped_keyed, encoded_files, read_files, tray, words_area, Attached, Kept, KeptTag};
use super::format::{draw, enter_in, sent_text};
use super::offer::{at_rows, mention_of, offer_list_at, put_pick, token_at, Offer, Row};
use super::JobView;
use crate::fleet::FleetConnection;
use crate::look::ActiveLook;

/// What `@` can offer, read by the host when the note opens: Fleet and the repository for the files, and the Jobs by id and title.
#[derive(Default)]
pub(crate) struct Scope {
    pub fleet: Option<Entity<FleetConnection>>,
    pub manifest: Option<String>,
    pub jobs: Vec<(String, String)>,
    /// The repository's files on mock data, narrowed as Fleet narrows them.
    pub mock_files: Vec<String>,
}

/// The note changed what it draws in a way its host's card must measure again: a tray came or went, or a list opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Resized;

pub(crate) struct RichNote {
    input: Entity<TextareaState>,
    /// What the box is called to VoiceOver and a walk.
    name: SharedString,
    files: Vec<Attached>,
    next_key: u64,
    offer: Option<Offer>,
    scope: Scope,
    /// The last file search answered, and the query it answered.
    found: Option<(String, Vec<String>)>,
    lookup: Option<Task<()>>,
    _typing: Subscription,
}

impl EventEmitter<Resized> for RichNote {}

/// The words and chips a text area holds, as a draft keeps them.
pub(crate) fn kept_of(input: &Entity<TextareaState>, cx: &App) -> Kept {
    let input = input.read(cx);
    Kept {
        text: input.value().to_string(),
        tags: input
            .tokens()
            .iter()
            .map(|span| KeptTag { start: span.range().start, end: span.range().end, id: span.token().id().to_string(), label: span.token().label().to_string() })
            .collect(),
    }
}

impl RichNote {
    /// A note over `input`, which keeps being the host's own text area.
    pub fn new(input: Entity<TextareaState>, name: impl Into<SharedString>, scope: Scope, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let typing = cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| {
            if let InputEvent::Change = event {
                draw(input, window, cx);
                this.refresh_offer(window, cx);
            }
        });
        RichNote { input, name: name.into(), files: Vec::new(), next_key: 0, offer: None, scope, found: None, lookup: None, _typing: typing }
    }

    pub fn input(&self) -> &Entity<TextareaState> {
        &self.input
    }

    /// The words and chips the box holds, as a draft keeps them.
    pub fn kept(&self, cx: &App) -> Kept {
        kept_of(&self.input, cx)
    }

    /// The words as a message sends them, trimmed, with a blank line where a quote or a list ends in words.
    pub fn sent(&self, cx: &App) -> String {
        sent_text(self.input.read(cx).value().trim())
    }

    /// The files attached and the Jobs tagged, as an answer carries them.
    pub fn extras(&self, cx: &App) -> AnswerExtras {
        AnswerExtras {
            attachments: encoded_files(self.files.clone()),
            mentions: self.kept(cx).tags.iter().filter_map(|tag| mention_of(&tag.id, &tag.label)).collect(),
        }
    }

    /// Whether a file is attached: the box holds something besides words.
    pub fn holds_files(&self) -> bool {
        !self.files.is_empty()
    }

    /// A list is open over the words, so Esc is the list's.
    pub fn offering(&self) -> bool {
        self.offer.is_some()
    }

    /// Put the list away: Esc where one is open.
    pub fn close_offer(&mut self, cx: &mut Context<Self>) {
        self.put_offer(None, cx);
    }

    // ---- what `@` offers -----------------------------------------------------------------

    fn refresh_offer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (text, cursor) = {
            let input = self.input.read(cx);
            (input.value().to_string(), input.cursor())
        };
        let Some(('@', from, query)) = token_at(&text, cursor) else {
            self.put_offer(None, cx);
            return;
        };
        self.want_files(&query, window, cx);
        let files = self.found.as_ref().filter(|(asked, _)| *asked == query).map(|(_, paths)| paths.clone());
        let loading = files.is_none();
        let stale = files.or_else(|| self.found.as_ref().map(|(_, paths)| paths.clone())).unwrap_or_default();
        let rows: Vec<Row> = at_rows(&self.scope.jobs, stale, &query);
        if rows.is_empty() && !loading {
            self.put_offer(None, cx);
            return;
        }
        let label = "Tag a Job or a file";
        let current = self.offer.as_ref().map_or(0, |held| held.current.min(rows.len().saturating_sub(1)));
        self.put_offer(Some(Offer { label, rows, current, from, to: cursor, query, loading }), cx);
    }

    fn put_offer(&mut self, offer: Option<Offer>, cx: &mut Context<Self>) {
        let moved = self.offer.is_some() != offer.is_some();
        self.offer = offer;
        if moved {
            cx.emit(Resized);
        }
        cx.notify();
    }

    /// Ask Fleet for the files a query names, 150 ms after the last key.
    fn want_files(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.found.as_ref().is_some_and(|(asked, _)| asked == query) {
            return;
        }
        let Some(fleet) = self.scope.fleet.clone() else {
            // Mock data: the repository's files are the fixture's, narrowed as Fleet narrows them.
            let lowered = query.to_lowercase();
            let paths = self.scope.mock_files.iter().filter(|path| path.to_lowercase().contains(&lowered)).take(50).cloned().collect();
            self.found = Some((query.to_string(), paths));
            return;
        };
        let (Some(manifest), Some(port)) = (self.scope.manifest.clone(), fleet.read(cx).held().port()) else { return };
        let path = format!("/manifest/files?q={}&manifest_id={}", super::wire::encoded(query), super::wire::encoded(&manifest));
        let asked = query.to_string();
        self.lookup = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(150)).await;
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::get(port, &path))?;
                    if !(200..300).contains(&status) {
                        return Err(super::wire::reason(status, &bytes));
                    }
                    ipc::decode::<ipc::FilesFound>("FilesFound", &bytes).map_err(|why| why.to_string())
                })
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                if let Ok(found) = answer {
                    this.found = Some((asked, found.paths));
                    this.refresh_offer(window, cx);
                }
            });
        }));
    }

    /// A key while a list is open is the list's, and the box does not see it. True where taken.
    fn offer_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(offer) = self.offer.as_mut() else { return false };
        let last = offer.rows.len().saturating_sub(1);
        let was = offer.current;
        match key {
            "down" => offer.current = (offer.current + 1).min(last),
            "up" => offer.current = offer.current.saturating_sub(1),
            "escape" => self.put_offer(None, cx),
            "enter" | "tab" => {
                let current = offer.current;
                self.choose(current, window, cx);
            }
            _ => return false,
        }
        if self.offer.as_ref().is_some_and(|offer| offer.current != was) && matches!(key, "up" | "down") {
            crate::ui_sounds::moved();
        }
        cx.notify();
        true
    }

    /// A row chosen: a Job becomes a chip, a file its path in the words.
    fn choose(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(offer) = self.offer.take() else { return };
        cx.emit(Resized);
        let Some(row) = offer.rows.get(at).cloned() else { return };
        let range = offer.from..offer.to;
        self.input.update(cx, |input, cx| {
            input.focus(window, cx);
            put_pick(input, row.pick, range, window, cx);
        });
        cx.notify();
    }

    /// The keys the text box would take, which a list over it takes first.
    fn list_key(&mut self, key: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if self.offer_key(key, window, cx) {
            cx.stop_propagation();
        }
    }

    // ---- attachments ---------------------------------------------------------------------

    /// Files named by a drop or a paste are read off the main thread and attached.
    pub fn take_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        read_files(paths, cx, |this, name, media_type, bytes, cx| this.add_file(name, media_type, bytes, cx));
    }

    pub fn add_file(&mut self, name: String, media_type: String, bytes: Vec<u8>, cx: &mut Context<Self>) {
        self.next_key += 1;
        self.files.push(attached(self.next_key, name, media_type, bytes));
        cx.emit(Resized);
        cx.notify();
    }

    fn remove_file(&mut self, key: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.files.retain(|one| one.key != key);
        // The caret goes back to the box where the tile was.
        self.input.update(cx, |input, cx| input.focus(window, cx));
        cx.emit(Resized);
        cx.notify();
    }

    /// A paste with a picture or files on the clipboard attaches them; a long text becomes a file.
    pub fn paste(&mut self, item: &ClipboardItem, cx: &mut Context<Self>) -> bool {
        let taken = clipped_keyed(item, &mut self.next_key);
        for (name, media_type, bytes) in taken.files.iter().cloned() {
            self.add_file(name, media_type, bytes, cx);
        }
        let any = taken.taken();
        if !taken.paths.is_empty() {
            self.take_paths(taken.paths, cx);
        }
        any
    }
}

impl gpui::Render for RichNote {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let name = self.name.clone();
        let this = cx.entity().downgrade();
        let words = {
            let view = this.clone();
            words_area(&self.input, look, move |item, _, cx| view.update(cx, |this, cx| this.paste(item, cx)).unwrap_or(false)).aria_label(name)
        };
        let remove = |key: u64| -> Box<dyn Fn(&ClickEvent, &mut Window, &mut App)> {
            let this = this.clone();
            Box::new(move |_, window, cx| {
                let _ = this.update(cx, |this, cx| this.remove_file(key, window, cx));
            })
        };
        let offer = self.offer.as_ref().map(|offer| {
            offer_list_at(
                offer,
                look,
                |at| {
                    let this = cx.entity().downgrade();
                    Box::new(move |_, window, cx| {
                        let _ = this.update(cx, |this, cx| this.choose(at, window, cx));
                    })
                },
                false,
            )
        });
        let tiles = (!self.files.is_empty()).then(|| tray(&self.files, Vec::new(), look, remove));
        div()
            .id("rich-note")
            .w_full()
            .flex()
            .flex_col()
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| this.take_paths(paths.paths().to_vec(), cx)))
            // A list over the words takes these keys before the text area does, as the Job's box lets it.
            .capture_action(cx.listener(|this, action: &Enter, window, cx| {
                if action.secondary {
                    return;
                }
                if !action.shift && this.offer_key("enter", window, cx) {
                    cx.stop_propagation();
                } else if enter_in(&this.input, action.shift, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, window, cx| this.list_key("up", window, cx)))
            .capture_action(cx.listener(|this, _: &MoveDown, window, cx| this.list_key("down", window, cx)))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| this.list_key("escape", window, cx)))
            .capture_action(cx.listener(|this, _: &IndentInline, window, cx| this.list_key("tab", window, cx)))
            .children(offer)
            .children(tiles)
            .child(words)
    }
}

impl JobView {
    /// What `@` can offer in a box opened on the shown Job: its repository's files and the other Jobs that are not over.
    pub(super) fn note_scope(&self, cx: &App) -> Scope {
        let Some(id) = self.shown.as_ref() else { return Scope::default() };
        let manifest = self.record(id, cx).and_then(|job| job.manifest_id.as_ref().map(|id| id.as_str().to_string()));
        let jobs = self
            .list
            .read(cx)
            .records()
            .iter()
            .filter(|job| job.id.as_str() != id.as_str() && job.state != ipc::SessionState::Ended)
            .filter_map(|job| job.title.as_ref().map(|title| (job.id.as_str().to_string(), title.clone())))
            .collect();
        Scope { fleet: self.fleet.clone(), manifest, jobs, mock_files: self.composer.catalog.mock_files.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_view::acts::{Mode, SHEET};
    use crate::job_view::offer::Pick;
    use crate::job_view::parts_fold::rooted_job;
    use gpui::{AppContext as _, Image, ImageFormat, TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext, scope: Scope) -> (Entity<RichNote>, &mut VisualTestContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::look::init(cx);
            crate::keymap::install(&crate::keymap::Rebinding::default(), cx);
        });
        cx.add_window_view(move |window, cx| {
            let input = cx.new(|cx| TextareaState::new(window, cx).auto_grow(3, 8));
            RichNote::new(input, "What should change?", scope, window, cx)
        })
    }

    fn picture() -> ClipboardItem {
        ClipboardItem::new_image(&Image::from_bytes(ImageFormat::Png, vec![1, 2, 3]))
    }

    #[gpui::test]
    fn a_pasted_picture_is_attached_to_the_box_and_sent_as_an_upload(cx: &mut TestAppContext) {
        let (note, cx) = open(cx, Scope::default());
        let taken = note.update(cx, |note, cx| note.paste(&picture(), cx));
        assert!(taken, "a picture on the clipboard is taken, not typed");
        note.read_with(cx, |note, cx| {
            assert!(note.holds_files());
            let uploads = note.extras(cx).attachments;
            assert_eq!(uploads.len(), 1);
            assert_eq!((uploads[0].media_type.as_str(), uploads[0].data.as_str()), ("image/png", "AQID"));
        });
    }

    #[gpui::test]
    fn an_at_pick_adds_a_tag_that_is_sent_as_a_mention(cx: &mut TestAppContext) {
        let scope = Scope { jobs: vec![("job-2".into(), "Retry the sync".into())], ..Scope::default() };
        let (note, cx) = open(cx, scope);
        note.update_in(cx, |note, window, cx| {
            note.input.update(cx, |input, cx| {
                input.set_value("see @Ret", window, cx);
                let end = input.value().len();
                input.set_selected_range(end..end, cx);
            });
            note.refresh_offer(window, cx);
            let offer = note.offer.as_ref().expect("a list opens under @");
            assert!(matches!(offer.rows.first().map(|row| &row.pick), Some(Pick::Tag { id, .. }) if id == "job-2"), "the Job whose title holds the word is offered");
            note.choose(0, window, cx);
        });
        let mentions = note.read_with(cx, |note, cx| note.extras(cx).mentions);
        assert_eq!(mentions.len(), 1, "{mentions:?}");
        assert_eq!((mentions[0].kind, mentions[0].id.as_str(), mentions[0].title.as_str()), (ipc::TagKind::Session, "job-2", "Retry the sync"));
    }

    #[gpui::test]
    fn a_formatted_word_reaches_the_sent_words(cx: &mut TestAppContext) {
        let (note, cx) = open(cx, Scope::default());
        note.update_in(cx, |note, window, cx| {
            note.input.update(cx, |input, cx| input.set_value("> a held line\nthen my words", window, cx));
            // What a keystroke's change does.
            draw(&note.input, window, cx);
            assert_eq!(note.kept(cx).tags.len(), 1, "the quote is drawn as a quote");
            assert_eq!(note.sent(cx), "> a held line\n\nthen my words", "words after a quote go apart from it, as a message's do");
            assert!(note.extras(cx).mentions.is_empty(), "a quote's mark is no tag");
        });
    }

    #[gpui::test]
    fn what_a_card_box_holds_reaches_the_answer_its_card_builds(cx: &mut TestAppContext) {
        let (view, cx) = rooted_job(cx, "job-questions", "mock-01", gpui::size(gpui::px(1400.), gpui::px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_note(SHEET, "card-1", "What should change?", window, cx));
        let note = view.read_with(cx, |view, _| view.pages.get("mock-01").and_then(|page| page.cards.notes.get("card-1:Note").cloned())).expect("the box opens as a rich one");
        note.update_in(cx, |note, window, cx| {
            assert!(note.paste(&picture(), cx));
            note.input.update(cx, |input, cx| input.set_value("- one\nand more", window, cx));
            draw(&note.input, window, cx);
        });
        let (words, extras) = view.update(cx, |view, cx| (view.typed_in("card-1", Mode::Note, cx), view.extras_of("card-1", Mode::Note, cx)));
        assert_eq!(words, "- one\n\nand more", "the card reads the words as a message sends them");
        assert_eq!(extras.attachments.len(), 1, "the card reads the picture");
        // The answer a box builds is these words and these extras.
        let body = crate::job_view::reader::verdict("mock-01", "document:notes", ipc::ItemHow::SendBack, Vec::new(), extras);
        let wire = ipc::encode(&body).expect("encodes");
        assert!(wire.contains(r#""media_type":"image/png""#) && wire.contains(r#""data":"AQID""#), "{wire}");
    }
}
