//! Make it a Document: an open draft that proposes no change becomes a page he reads and notes
//! (`POST /sessions/draft/document`). Fleet writes the page, asks him to read it, and tells the Job; the
//! thread goes to the page's card once Fleet's thread holds it.

use gpui::{Context, SharedString, Window};
use ipc::{ConvertDraftToDocument, DraftDocument, SessionId, ThreadItem};

use super::JobView;

/// Where the page's card stands in the thread: Fleet asks for it as `document:<path>`, and the card
/// carries the path as its page.
pub(super) fn made_card_at(items: &[ThreadItem], path: &str) -> Option<usize> {
    items.iter().position(|item| matches!(item, ThreadItem::Document(card) if card.page == path))
}

impl JobView {
    /// The press: the draft's row `id` is sent, and a refusal is said on the card. The card itself is
    /// left as it is; Fleet's row says what became of the draft.
    pub(super) fn make_document(&mut self, ix: usize, key: String, id: &str, title: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        if let Some(page) = self.pages.get_mut(&job) {
            page.cards.said.remove(&key);
        }
        let Some(fleet) = self.fleet.clone() else {
            self.mock_document(&job, title, cx);
            return;
        };
        let Some(port) = fleet.read(cx).held().port() else {
            self.undo(&job, ix, &key, "Fleet is not connected".into(), cx);
            return;
        };
        let body = ipc::encode(&ConvertDraftToDocument { session_id: SessionId::carried(job.to_string()), draft_id: format!("draft:{id}") }).map_err(|why| format!("{why:?}"));
        cx.spawn_in(window, async move |this, cx| {
            let answer = cx
                .background_executor()
                .spawn(async move {
                    let body = body?;
                    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                    let (status, bytes) = runtime.block_on(crate::transport::post(port, "/sessions/draft/document", body))?;
                    if (200..300).contains(&status) {
                        return ipc::decode::<DraftDocument>("DraftDocument", &bytes).map_err(|why| why.to_string());
                    }
                    Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
                })
                .await;
            this.update(cx, |this, cx| match answer {
                Ok(made) => this.go_to_made(&job, made.path, cx),
                Err(why) => this.undo(&job, ix, &key, SharedString::from(why), cx),
            })
            .ok();
        })
        .detach();
    }

    /// The page is kept to go to, and gone to now where its card is already in the thread.
    fn go_to_made(&mut self, job: &SharedString, path: String, cx: &mut Context<Self>) {
        if let Some(page) = self.pages.get_mut(job) {
            page.made_pages.insert(path.clone());
            page.made = Some(path);
        }
        self.open_made(job, cx);
    }

    /// Run after each sync: the thread goes to the made page's card the first time it is there.
    pub(super) fn open_made(&mut self, job: &SharedString, cx: &mut Context<Self>) {
        let Some(path) = self.pages.get(job).and_then(|page| page.made.clone()) else { return };
        let Some(at) = self.thread(job, cx).and_then(|held| made_card_at(&held.items, &path)) else { return };
        if let Some(page) = self.pages.get_mut(job) {
            page.made = None;
            page.list.scroll_to(gpui::ListOffset { item_ix: at + page.lead, offset_in_item: gpui::px(0.) });
        }
        cx.notify();
    }

    /// Mock data: Fleet's card for the page arrives at once, under the draft's title.
    fn mock_document(&mut self, job: &SharedString, title: &str, cx: &mut Context<Self>) {
        let slug: String = title.split(|ch: char| !ch.is_ascii_alphanumeric()).filter(|part| !part.is_empty()).map(str::to_ascii_lowercase).collect::<Vec<_>>().join("-");
        let path = format!("/work/slots/slot-3/.armada/documents/{slug}.html");
        let at = ipc::Instant::carried(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        if let Some(held) = self.mock.get_mut(job.as_str()) {
            held.items.push(ThreadItem::Document(ipc::ThreadDocument {
                id: format!("document:{path}:r1"),
                at,
                page: path.clone(),
                title: title.to_string(),
                revision: 1,
                asked: true,
                settled: None,
            }));
            held.version += 1;
            let at = held.items.len() - 1;
            held.mark_dirty(at);
        }
        if let Some(page) = self.pages.get_mut(job) {
            page.made_pages.insert(path.clone());
            page.made = Some(path);
        }
        self.sync(job, cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(page: &str) -> ThreadItem {
        ThreadItem::Document(ipc::ThreadDocument {
            id: format!("document:{page}:r1"),
            at: ipc::Instant::carried("2026-11-10T22:00:00Z"),
            page: page.to_string(),
            title: "Notes".into(),
            revision: 1,
            asked: true,
            settled: None,
        })
    }

    #[test]
    fn the_made_page_is_found_by_its_path_and_only_once_its_card_is_there() {
        let items = vec![document("/w/.armada/documents/other.html"), document("/w/.armada/documents/notes.html")];
        assert_eq!(made_card_at(&items, "/w/.armada/documents/notes.html"), Some(1));
        assert_eq!(made_card_at(&items[..1], "/w/.armada/documents/notes.html"), None);
    }
}
