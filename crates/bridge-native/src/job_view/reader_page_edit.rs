//! Edit on a page: its words become editable where they stand in its web view, a block he changed is marked in his
//! colour as a draft's is, and Save sends the revision's own markup with his words in it (`web::editing`,
//! `save_artifact_edit`). The page is read back every two seconds and as he leaves Edit, and what he typed is kept
//! (`drafts.rs`) with the revision it was typed on, so Edit left half done opens as he left it on that revision.
//!
//! Leaving Edit, Discard and Save read the page again from Fleet, so it shows what Fleet holds and nothing stays editable.

use std::time::Duration;

use gpui::{Context, Entity, Hsla, Rgba, SharedString, Task, Window};
use ipc::{SaveArtifactEdit, SessionId};

use super::page_tab::PageTab;
use super::reader::{edit_key, Edit, Subject};
use super::reader_draw::RAct;
use super::JobView;
use crate::look::{token, ActiveLook};
use crate::web::editing::{self, Kept, Marks, Texts};

/// How often what he typed is read back from the page and kept.
const KEEPING: Duration = Duration::from_secs(2);

/// Edit on a page: the page he types in, and the reads that keep his words while it is on.
pub(super) struct PageEdit {
    pub tab: Entity<PageTab>,
    pub revision: u32,
    _keeping: Task<()>,
}

/// The kept value of an Edit left half done, none where it changed no words.
pub(super) fn kept_of(revision: u32, texts: &Texts) -> String {
    if !texts.changed() {
        return String::new();
    }
    ipc::encode(&Kept { revision, texts: texts.now.clone() }).unwrap_or_default()
}

/// What was kept for a page's Edit, where it was typed on `revision`.
pub(super) fn kept_read(kept: &str, revision: u32) -> Option<Vec<String>> {
    let kept: Kept = ipc::decode("kept page words", kept.as_bytes()).ok()?;
    (kept.revision == revision).then_some(kept.texts)
}

/// The body of `save_artifact_edit` for his words on revision `revision` of the page, and the page it sends: `source` with
/// his words in it. Refused, in words for him, where the words do not match the markup.
pub(super) fn save_body(job: &str, artifact: &str, revision: u32, source: &str, texts: &Texts) -> Result<(String, String), String> {
    let html = editing::spliced(source, texts)?;
    let body = ipc::encode(&SaveArtifactEdit { session_id: SessionId::carried(job.to_string()), artifact: artifact.to_string(), revision, text: html.clone() }).map_err(|why| format!("{why:?}"))?;
    Ok((body, html))
}

/// A colour as a stylesheet reads it.
fn css(color: Hsla) -> String {
    let rgba: Rgba = color.into();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u8;
    format!("rgba({},{},{},{:.3})", byte(rgba.r), byte(rgba.g), byte(rgba.b), rgba.a.clamp(0., 1.))
}

impl JobView {
    /// Edit on or off, Save and Discard, on a page.
    pub(super) fn page_edit_act(&mut self, id: &SharedString, name: &str, act: RAct, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let editing = self.reader_of(id, name).is_some_and(|held| held.edit.is_some());
        match act {
            RAct::Edit if !editing && !subject.closed => self.page_edit_start(id, name, subject, window, cx),
            RAct::Edit | RAct::Discard if editing => self.page_edit_end(id, name, false, true, window, cx),
            RAct::Save if editing => self.page_edit_save(id, name, subject, window, cx),
            _ => {}
        }
    }

    fn page_edit_start(&mut self, id: &SharedString, name: &str, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        let revision = tab.read(cx).shown();
        if self.reader_of(id, name).is_none_or(|held| held.viewing.is_some_and(|at| at != subject.newest)) || revision != subject.newest {
            return;
        }
        let kept = kept_read(&self.draft_words(&edit_key(id, &subject.artifact)), revision);
        let look = cx.look();
        let marks = Marks { edge: css(look.color(token::YOU)), fill: css(look.color(token::YOU).opacity(0.12)), line: css(look.color(token::ACCENT).opacity(0.4)) };
        let asked = tab.read(cx).ask(&editing::start(&marks, kept.as_deref()), cx);
        let keeping = self.page_keeping(id, &subject.artifact, revision, &tab, cx);
        if let Some(held) = self.reader_mut(id, name) {
            held.edit = Some(Edit::page(PageEdit { tab: tab.clone(), revision, _keeping: keeping }));
            held.said = None;
        }
        cx.notify();
        let (job, reader) = (id.clone(), name.to_string());
        cx.spawn_in(window, async move |this, cx| {
            let started = editing::started(asked.recv().await.ok().flatten().as_deref());
            let _ = this.update_in(cx, |this, window, cx| {
                let said = match started {
                    None => "The page has not loaded yet, so its words cannot be edited",
                    Some(0) => "This page has no words to edit",
                    Some(_) => {
                        tab.read(cx).take_keys(cx);
                        return;
                    }
                };
                this.page_edit_end(&job, &reader, true, true, window, cx);
                if let Some(held) = this.reader_mut(&job, &reader) {
                    held.said = Some(said.into());
                }
            });
        })
        .detach();
    }

    /// What he types is read back from the page and kept, every two seconds while Edit is on.
    fn page_keeping(&self, id: &SharedString, artifact: &str, revision: u32, tab: &Entity<PageTab>, cx: &Context<Self>) -> Task<()> {
        let (key, tab) = (edit_key(id, artifact), tab.downgrade());
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(KEEPING).await;
            let Ok(asked) = tab.read_with(cx, |tab, cx| tab.ask(&editing::read(), cx)) else { return };
            let texts = editing::texts_of(asked.recv().await.ok().flatten().as_deref());
            let kept = this.update(cx, |this, cx| {
                if let Some(texts) = texts {
                    this.keep_words(&key, &kept_of(revision, &texts), cx);
                }
            });
            if kept.is_err() {
                return;
            }
        })
    }

    /// Edit on the page ends. What he typed is read back and kept where `keep`, else let go; where `again` the page is read
    /// again from Fleet, so it shows Fleet's words and nothing in it is editable.
    pub(super) fn page_edit_end(&mut self, id: &SharedString, name: &str, keep: bool, again: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        let Some(page) = held.edit.take().and_then(|edit| edit.page) else { return };
        held.saving = false;
        held.said = None;
        let Some(artifact) = self.reader_artifact(id, name, cx) else { return };
        let key = edit_key(id, &artifact);
        page.tab.read(cx).release_keys(cx);
        cx.notify();
        if !keep {
            self.keep_words(&key, "", cx);
            if again {
                self.page_again(&page.tab, window, cx);
            }
            return;
        }
        let asked = page.tab.read(cx).ask(&editing::read(), cx);
        cx.spawn_in(window, async move |this, cx| {
            let texts = editing::texts_of(asked.recv().await.ok().flatten().as_deref());
            let _ = this.update_in(cx, |this, window, cx| {
                if let Some(texts) = texts {
                    this.keep_words(&key, &kept_of(page.revision, &texts), cx);
                }
                if again {
                    this.page_again(&page.tab, window, cx);
                }
            });
        })
        .detach();
    }

    /// The page the tab draws, read again from Fleet.
    fn page_again(&mut self, tab: &Entity<PageTab>, window: &mut Window, cx: &mut Context<Self>) {
        tab.update(cx, |tab, cx| {
            let shown = tab.shown();
            tab.show_revision(shown, cx);
        });
        self.fetch_page(tab.clone(), window, cx);
    }

    /// Save: the page says what he typed, the revision's own markup is read from Fleet, and his words go into it.
    fn page_edit_save(&mut self, id: &SharedString, name: &str, subject: &Subject, window: &mut Window, cx: &mut Context<Self>) {
        let Some(held) = self.reader_mut(id, name) else { return };
        let Some((tab, revision)) = held.edit.as_ref().and_then(|edit| edit.page.as_ref()).map(|page| (page.tab.clone(), page.revision)) else { return };
        if held.saving {
            return;
        }
        held.saving = true;
        held.said = None;
        cx.notify();
        let asked = tab.read(cx).ask(&editing::read(), cx);
        let (job, reader, artifact) = (id.clone(), name.to_string(), subject.artifact.clone());
        cx.spawn_in(window, async move |this, cx| {
            let texts = editing::texts_of(asked.recv().await.ok().flatten().as_deref());
            let _ = this.update_in(cx, |this, window, cx| this.page_edit_read(&job, &reader, &artifact, revision, texts, window, cx));
        })
        .detach();
    }

    #[allow(clippy::too_many_arguments)]
    fn page_edit_read(&mut self, id: &SharedString, name: &str, artifact: &str, revision: u32, texts: Option<Texts>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(texts) = texts else {
            self.page_edit_refused(id, name, "The page did not say what was typed in it, so nothing was saved".into(), cx);
            return;
        };
        // Words that say what the page says keep nothing.
        if !texts.changed() {
            self.page_edit_end(id, name, false, true, window, cx);
            return;
        }
        if self.fleet.is_none() {
            let his = self.row_tabs.get(id).and_then(|tabs| tabs.mock_kept.get(artifact)).map(|kept| kept.his.clone()).unwrap_or_default();
            match crate::mock::page_source(artifact, revision, &his) {
                Ok(source) => self.page_edit_send(id, name, artifact, revision, source, texts, window, cx),
                Err(why) => self.page_edit_refused(id, name, why.into(), cx),
            }
            return;
        }
        let path = format!("/sessions/artifacts/page/source?session_id={}&artifact={}&revision={revision}", super::wire::encoded(id), super::wire::encoded(artifact));
        let (job, reader, artifact) = (id.clone(), name.to_string(), artifact.to_string());
        self.get(
            path,
            window,
            cx,
            Box::new(move |this, answer, window, cx| match answer.and_then(|bytes| String::from_utf8(bytes).map_err(|_| "Fleet's page is not text".to_string())) {
                Ok(source) => this.page_edit_send(&job, &reader, &artifact, revision, source, texts, window, cx),
                Err(why) => this.page_edit_refused(&job, &reader, why.into(), cx),
            }),
        );
    }

    /// His words go into the markup off the main thread, and the page is sent.
    #[allow(clippy::too_many_arguments)]
    fn page_edit_send(&mut self, id: &SharedString, name: &str, artifact: &str, revision: u32, source: String, texts: Texts, window: &mut Window, cx: &mut Context<Self>) {
        let (job, reader, artifact) = (id.clone(), name.to_string(), artifact.to_string());
        let made = {
            let (job, artifact) = (job.to_string(), artifact.clone());
            cx.background_executor().spawn(async move { save_body(&job, &artifact, revision, &source, &texts) })
        };
        cx.spawn_in(window, async move |this, cx| {
            let made = made.await;
            let _ = this.update_in(cx, |this, window, cx| match made {
                Ok((body, html)) => this.page_edit_post(&job, &reader, &artifact, body, html, window, cx),
                Err(why) => this.page_edit_refused(&job, &reader, why.into(), cx),
            });
        })
        .detach();
    }

    #[allow(clippy::too_many_arguments)]
    fn page_edit_post(&mut self, id: &SharedString, name: &str, artifact: &str, body: String, html: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.fleet.is_none() {
            self.edited_here(id, artifact);
            let kept = self.row_tabs.entry(id.clone()).or_default().mock_kept.entry(artifact.to_string()).or_default();
            kept.his.push(html);
            let newest = self.reader_page(id, name).map(|tab| tab.read(cx).shown() + 1);
            self.page_edit_kept(id, name, newest, window, cx);
            return;
        }
        let (job, reader) = (id.clone(), name.to_string());
        self.post(
            "/sessions/artifacts/edit".to_string(),
            Ok(body),
            window,
            cx,
            Box::new(move |this, answer, window, cx| match answer {
                Ok(bytes) => {
                    let newest = ipc::decode::<ipc::JobArtifact>("artifact", &bytes).ok().and_then(|kept| kept.revisions.last().map(|last| last.number));
                    this.page_edit_kept(&job, &reader, newest, window, cx);
                }
                Err(why) => this.page_edit_refused(&job, &reader, why.into(), cx),
            }),
        );
    }

    /// Fleet kept his words as the next revision: Edit is over, what was kept for it goes, and the page draws that revision.
    fn page_edit_kept(&mut self, id: &SharedString, name: &str, newest: Option<u32>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(tab) = self.reader_page(id, name) else { return };
        let shown = tab.read(cx).shown();
        self.page_edit_end(id, name, false, newest.is_none_or(|newest| newest == shown), window, cx);
        if let Some(newest) = newest.filter(|newest| *newest != shown) {
            tab.update(cx, |tab, cx| tab.show_revision(newest, cx));
            self.fetch_page(tab, window, cx);
        }
    }

    /// Save was refused: Edit stays on with his words, and the reason stands under the page.
    fn page_edit_refused(&mut self, id: &SharedString, name: &str, why: SharedString, cx: &mut Context<Self>) {
        if let Some(held) = self.reader_mut(id, name) {
            held.saving = false;
            held.said = Some(why);
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = include_str!("../../fixtures/page-actions-source.html");

    #[test]
    fn save_sends_the_revisions_own_markup_with_his_words_in_it() {
        let was = editing::words_of(PAGE);
        let mut now = was.clone();
        let at = was.iter().position(|one| one == "Behind main").expect("a table heading");
        now[at] = "Days behind".into();
        let (body, html) = save_body("job-1", "design", 2, PAGE, &Texts { was, now }).expect("his words fit the page");
        let sent: SaveArtifactEdit = ipc::decode("save body", body.as_bytes()).expect("the body is a SaveArtifactEdit");
        assert_eq!((sent.session_id.as_str(), sent.artifact.as_str(), sent.revision), ("job-1", "design", 2));
        assert_eq!(sent.text, html);
        assert_eq!(sent.text, PAGE.replacen("<th>Behind main</th>", "<th>Days behind</th>", 1), "the markup, the actions and the votes are as the Job wrote them");
    }

    #[test]
    fn save_is_refused_in_words_where_the_page_does_not_match_its_markup() {
        let texts = Texts { was: vec!["Not this page".into()], now: vec!["Changed".into()] };
        assert!(save_body("job-1", "design", 2, PAGE, &texts).is_err_and(|why| why.contains("could not be matched")));
    }

    #[test]
    fn what_he_typed_is_kept_with_its_revision_and_read_back_only_on_it() {
        let texts = Texts { was: vec!["One".into(), "Two".into()], now: vec!["One".into(), "Two more".into()] };
        let kept = kept_of(2, &texts);
        assert_eq!(kept_read(&kept, 2), Some(texts.now.clone()));
        assert_eq!(kept_read(&kept, 3), None, "words typed on another revision are not put into this one");
        assert_eq!(kept_of(2, &Texts { was: texts.was.clone(), now: texts.was.clone() }), "", "no change keeps nothing");
        assert_eq!(kept_read("", 2), None);
    }

    #[test]
    fn the_mock_serves_his_saved_page_as_the_next_revision_with_the_kit_on_it() {
        let was = editing::words_of(PAGE);
        let mut now = was.clone();
        let at = was.iter().position(|one| one == "Behind main").expect("a table heading");
        now[at] = "Days behind".into();
        let (_, html) = save_body("job-1", "design", 2, PAGE, &Texts { was, now }).expect("fits");
        let his = vec![html.clone()];
        assert_eq!(crate::mock::page_source("design", 3, &his), Ok(html));
        let sealed = crate::mock::sealed_revision("design", 3, &his, &[], &[]).expect("revision 3 is his");
        assert!(sealed.contains("<th>Days behind</th>") && sealed.contains(r#""revision":3"#) && sealed.contains("window.armadaHost = hear"), "his words, sealed with the kit");
        assert!(crate::mock::sealed_revision("design", 4, &his, &[], &[]).is_err());
    }
}
