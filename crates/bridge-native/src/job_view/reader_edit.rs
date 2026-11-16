//! Edit on a draft: its words become a box for each block, which he types in. A block he changed is marked in his
//! colour, the header fills with it, and Save keeps the words as the next revision, his (`save_artifact_edit`).
//!
//! What he typed is kept as he types it (`drafts.rs`), so Edit left half done is there when he comes back. A page's
//! words are inside its web view and are not edited here (`reader_page.rs`).

use gpui::{AppContext as _, Context, SharedString, Window};
use gpui_component::input::{InputEvent, TextareaState};
use ipc::{SaveArtifactEdit, SessionId};

use super::reader::{edit_key, text_of, Edit, Subject};
use super::reader_draw::RAct;
use super::JobView;

/// What an edited draft keeps between runs: the words of each block.
pub(super) fn words_kept(words: &[String]) -> String {
    ipc::encode(&words).unwrap_or_default()
}

pub(super) fn words_read(kept: &str) -> Vec<String> {
    if kept.is_empty() {
        return Vec::new();
    }
    ipc::decode::<Vec<String>>("kept words", kept.as_bytes()).unwrap_or_default()
}

/// Whether the blocks read the same as they did, white space aside.
pub(super) fn unchanged(words: &[String], original: &[String]) -> bool {
    let squeeze = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    squeeze(&text_of(words)) == squeeze(&text_of(original))
}

impl JobView {
    /// Edit on or off, Save and Discard.
    pub(super) fn reader_edit_act(&mut self, id: &SharedString, name: &str, act: RAct, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let editing = self.reader_of(id, name).is_some_and(|held| held.edit.is_some());
        match act {
            RAct::Edit if !editing && !subject.closed => self.reader_edit_start(id, name, subject, window, cx),
            RAct::Edit | RAct::Discard if editing => self.reader_edit_stop(id, name, subject, cx),
            RAct::Save if editing => self.reader_edit_save(id, name, subject, window, cx),
            _ => {}
        }
    }

    fn reader_edit_start(&mut self, id: &SharedString, name: &str, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_of(id, name) else { return };
        if held.viewing.is_some_and(|at| at != subject.newest) || held.blocks.is_empty() {
            return;
        }
        let original: Vec<String> = held.blocks.iter().map(|one| one.block.source.clone()).collect();
        let kept = words_read(&self.draft_words(&edit_key(id, &subject.artifact)));
        let words = if kept.len() == original.len() { kept } else { original.clone() };
        let mut areas = Vec::new();
        let mut typing = Vec::new();
        for text in &words {
            let area = cx.new(|cx| TextareaState::new(window, cx).auto_grow(1, 16));
            area.update(cx, |area, cx| area.set_value(text.clone(), window, cx));
            let (job, reader) = (id.clone(), name.to_string());
            typing.push(cx.subscribe_in(&area, window, move |this, _, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    this.reader_edit_changed(&job, &reader, cx);
                }
            }));
            areas.push(area);
        }
        let first = held.blocks.iter().position(|one| one.block.kind == super::reader::BlockKind::Text).and_then(|at| areas.get(at).cloned());
        if let Some(held) = self.reader_mut(id, name) {
            held.edit = Some(Edit::new(areas, original, typing));
            held.said = None;
        }
        if let Some(area) = first {
            area.update(cx, |area, cx| area.focus(window, cx));
        }
        cx.notify();
    }

    /// His words are kept as he types, and gone when they say what the draft did.
    fn reader_edit_changed(&mut self, id: &SharedString, name: &str, cx: &mut Context<Self>) {
        let Some(edit) = self.reader_of(id, name).and_then(|held| held.edit.as_ref()) else { return };
        let (words, original) = (edit.words(cx), edit.original.clone());
        let Some(artifact) = self.reader_artifact(id, name, cx) else { return };
        let kept = if unchanged(&words, &original) { String::new() } else { words_kept(&words) };
        self.keep_words(&edit_key(id, &artifact), &kept, cx);
        cx.notify();
    }

    fn reader_edit_stop(&mut self, id: &SharedString, name: &str, subject: &Subject, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, name) {
            held.edit = None;
            held.saving = false;
            held.said = None;
        }
        self.keep_words(&edit_key(id, &subject.artifact), "", cx);
        cx.notify();
    }

    fn reader_edit_save(&mut self, id: &SharedString, name: &str, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let Some(edit) = self.reader_of(id, name).and_then(|held| held.edit.as_ref()) else { return };
        let (words, original) = (edit.words(cx), edit.original.clone());
        // Words that say what the draft says keep nothing.
        if unchanged(&words, &original) {
            self.reader_edit_stop(id, name, subject, cx);
            return;
        }
        let text = text_of(&words);
        let body = ipc::encode(&SaveArtifactEdit { session_id: SessionId::carried(id.to_string()), artifact: subject.artifact.clone(), revision: subject.newest, text: text.clone() });
        if let Some(held) = self.reader_mut(id, name) {
            held.saving = true;
            held.said = None;
        }
        cx.notify();
        let (job, reader, subject) = (id.clone(), name.to_string(), (subject.artifact.clone(), subject.row.clone(), subject.newest));
        if self.fleet.is_none() {
            self.mock_edited(&job, &subject.0, subject.1.as_deref(), &text, cx);
            self.reader_edit_kept(&job, &reader, &subject.0, cx);
            return;
        }
        self.post(
            "/sessions/artifacts/edit".to_string(),
            body,
            window,
            cx,
            Box::new(move |this, answer, _, cx| match answer {
                Ok(_) => this.reader_edit_kept(&job, &reader, &subject.0, cx),
                Err(why) => {
                    if let Some(held) = this.reader_mut(&job, &reader) {
                        held.saving = false;
                        held.said = Some(why.into());
                    }
                    cx.notify();
                }
            }),
        );
    }

    /// Fleet kept his words as the next revision: Edit is over, and what was kept for it goes.
    fn reader_edit_kept(&mut self, id: &SharedString, name: &str, artifact: &str, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, name) {
            held.edit = None;
            held.saving = false;
            // Its blocks are cut again from the words Fleet now holds.
            held.text.clear();
            held.blocks.clear();
        }
        self.keep_words(&edit_key(id, artifact), "", cx);
        cx.notify();
    }

    /// Mock data: the draft's row holds his words, the words it replaced are what it was, and the artifact has a revision of his.
    fn mock_edited(&mut self, job: &SharedString, artifact: &str, row: Option<&str>, text: &str, cx: &mut Context<Self>) {
        self.edited_here(job, artifact);
        let Some(held) = self.mock.get_mut(job.as_str()) else { return };
        let at = held.items.iter().position(|item| matches!(item, ipc::ThreadItem::Row { row: ipc::SessionRow::Draft { id, .. } } if Some(id.as_str()) == row));
        let Some(at) = at else { return };
        if let ipc::ThreadItem::Row { row: ipc::SessionRow::Draft { text: now, before, .. } } = &mut held.items[at] {
            *before = Some(std::mem::replace(now, text.to_string()));
        }
        held.version += 1;
        held.mark_dirty(at);
        self.sync(job, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_he_typed_is_kept_as_one_value_and_read_back() {
        let words = vec!["# Head".to_string(), "Some words.".to_string()];
        assert_eq!(words_read(&words_kept(&words)), words);
        assert!(words_read("").is_empty());
        assert!(words_read("{").is_empty());
    }

    #[test]
    fn words_that_say_what_it_said_are_no_edit() {
        let was = vec!["Each Job draws a bar.".to_string(), "Last.".to_string()];
        assert!(unchanged(&["Each Job  draws a bar.".to_string(), "Last.".to_string()], &was), "white space is not an edit");
        assert!(!unchanged(&["Each Job draws a bar beside the title.".to_string(), "Last.".to_string()], &was));
        assert!(!unchanged(&["Each Job draws a bar.".to_string()], &was), "a block taken out is an edit");
    }
}
