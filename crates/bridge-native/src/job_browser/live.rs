//! The real page behind an act: a Job's Browser tab, and the rest of what `acts::Host` asks of the app.
//!
//! Everything WebKit is asked goes through `WebPage` (`crate::web`), on the main thread, and is awaited here. A page nobody is
//! looking at is in no window, where WebKit still loads it and runs its scripts: it is made at a usual size when an act first
//! reaches it.

use std::time::Duration;

use gpui::{AppContext, AsyncApp, AsyncWindowContext, Entity, FutureExt, WeakEntity};
use ipc::{AnswerBrowserRequest, WalkTarget};

use super::acts::{self, Host, Loaded, Page, MOST_RESULT, SETTLE};
use super::scripts::{self, EVALUATE_WORLD, FINDER_WORLD};
use super::tab::BrowserTab;
use crate::job_view::JobView;
use crate::log::Note;
use crate::web::WebPage;

/// A script of Bridge's own answers within this, or WebKit lost the page and the act is answered as failed.
const SCRIPT: Duration = Duration::from_secs(10);

/// An agent's expression may await a page's own work, which Fleet's hold ends first.
const EXPRESSION: Duration = Duration::from_secs(25);

/// What the app hands an act: the Job view, to find a Job's page, mark an act on it and open a page for a Job with none.
pub struct LiveHost {
    pub view: WeakEntity<JobView>,
    pub cx: AsyncWindowContext,
    /// Where Fleet is, for the answer. None: no Fleet is connected, so there is nothing to answer.
    pub port: Option<u16>,
}

/// One act's mark on the page it runs on, which goes dim again when the act ends, however it ends.
pub struct ActMark {
    view: WeakEntity<JobView>,
    cx: AsyncApp,
    job: String,
    id: u64,
}

impl Drop for ActMark {
    fn drop(&mut self) {
        let (job, id) = (self.job.clone(), self.id);
        let _ = self.view.update(&mut self.cx, |view, cx| view.browser_unmark(&job, id, cx));
    }
}

impl Host for LiveHost {
    type Page = LivePage;
    type Mark = ActMark;

    fn page(&self, job: &str) -> Option<LivePage> {
        let mut cx = self.cx.clone();
        let tab = self.view.update_in(&mut cx, |view, window, cx| view.browser_page(job, window, cx)).ok().flatten()?;
        Some(LivePage::new(tab, &mut (*cx).clone()))
    }

    fn acting(&self, job: &str, sentence: String) -> ActMark {
        let mut cx = self.cx.clone();
        let id = self.view.update_in(&mut cx, |view, window, cx| view.browser_mark(job, sentence.into(), window, cx)).unwrap_or_default();
        ActMark { view: self.view.clone(), cx: (*self.cx).clone(), job: job.to_string(), id }
    }

    fn open_closed(&self, job: &str, _url: &str) -> LivePage {
        let mut cx = self.cx.clone();
        let opened = self.view.update_in(&mut cx, |view, window, cx| view.browser_open(job, window, cx));
        let tab = match opened {
            Ok(tab) => tab,
            // The Job view is gone, which only an app that is closing sees: a page of its own that nothing shows.
            Err(_) => cx
                .update(|window, app| app.new(|cx| BrowserTab::new(job.to_string().into(), super::UNATTACHED.into(), Default::default(), window, cx)))
                .expect("a window to make a page in while an act runs"),
        };
        LivePage::new(tab, &mut (*cx).clone())
    }

    async fn answer(&self, body: AnswerBrowserRequest) {
        let Some(port) = self.port else { return };
        let Ok(encoded) = ipc::encode(&body) else { return };
        let id = body.request_id.clone();
        let answered = self
            .cx
            .background_executor()
            .spawn(async move {
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
                runtime.block_on(crate::transport::post(port, "/sessions/browser/answer", encoded))
            })
            .await;
        match answered {
            Ok((status, _)) if (200..300).contains(&status) => {}
            // Fleet gave the call up first (409), or refused an answer that did not fit (422): nothing more to send.
            Ok((status, _)) => Note::warn("Fleet refused the answer to a browser act").field("request", id).field("status", u32::from(status)).emit(),
            Err(why) => Note::warn("a browser act's answer did not reach Fleet").field("request", id).field("why", why).emit(),
        }
    }
}

/// A Job's Browser tab, as the page an act works on.
pub struct LivePage {
    tab: Entity<BrowserTab>,
    page: Entity<WebPage>,
    cx: AsyncApp,
}

impl LivePage {
    fn new(tab: Entity<BrowserTab>, cx: &mut AsyncApp) -> Self {
        let page = tab.update(cx, |tab, cx| {
            tab.ensure_made(cx);
            tab.page().clone()
        });
        LivePage { tab, page, cx: cx.clone() }
    }

    /// What WebKit answered, or none where it did not within `within`.
    async fn heard<T>(&self, rx: async_channel::Receiver<T>, within: Duration) -> Option<T> {
        match rx.recv().with_timeout(within, self.cx.background_executor()).await {
            Ok(Ok(answer)) => Some(answer),
            _ => None,
        }
    }

    /// Runs `body` in `world` and answers what it returned.
    async fn call(&self, world: &'static str, body: String, within: Duration) -> Result<Option<String>, String> {
        let rx = self.page.read_with(&self.cx, |page, _| page.call_async(world, &body));
        self.heard(rx, within).await.unwrap_or_else(|| Err("The page did not answer.".to_string()))
    }

    /// The finder, in the page where a page load has cleared it.
    async fn finder_ready(&self) -> Result<(), String> {
        let rx = self.page.read_with(&self.cx, |page, _| page.evaluate_in(FINDER_WORLD, &scripts::install()));
        match self.heard(rx, SCRIPT).await {
            Some(Some(done)) if done == "installed" => Ok(()),
            _ => Err("The page would not take the finder.".to_string()),
        }
    }
}

/// `text` cut to at most `most` bytes, at a character.
fn cut(mut text: String, most: usize) -> String {
    if text.len() > most {
        let mut at = most;
        while !text.is_char_boundary(at) {
            at -= 1;
        }
        text.truncate(at);
    }
    text
}

impl Page for LivePage {
    fn url(&self) -> String {
        let url = self.page.read_with(&self.cx, |page, _| page.url()).unwrap_or_default();
        // A Job browser made empty shows no address until a page is opened in it.
        if url == "about:blank" {
            String::new()
        } else {
            url
        }
    }

    fn title(&self) -> String {
        self.page.read_with(&self.cx, |page, _| page.title())
    }

    async fn load(&self, url: &str, patience: Duration) -> Loaded {
        let mut cx = self.cx.clone();
        let rx = self.tab.update(&mut cx, |tab, cx| tab.begin_load(url, cx));
        match self.heard(rx, patience).await {
            Some(Ok(())) => Loaded::Loaded,
            Some(Err(why)) => Loaded::Failed(why),
            None => Loaded::TimedOut,
        }
    }

    async fn find(&self, target: &WalkTarget, patience: Duration) -> Result<bool, String> {
        self.finder_ready().await?;
        let answer = self.call(FINDER_WORLD, scripts::arrive(target, patience.as_millis() as u64), patience + SCRIPT).await?;
        Ok(answer.as_deref() == Some("found"))
    }

    async fn press(&self) -> Result<bool, String> {
        Ok(self.call(FINDER_WORLD, scripts::press(), SCRIPT).await?.as_deref() == Some("done"))
    }

    async fn fill(&self, text: &str, submit: bool) -> Result<bool, String> {
        Ok(self.call(FINDER_WORLD, scripts::fill(text, submit), SCRIPT).await?.as_deref() == Some("done"))
    }

    async fn settle(&self) {
        self.cx.background_executor().timer(SETTLE).await;
    }

    async fn snapshot(&self) -> Result<String, String> {
        self.finder_ready().await?;
        Ok(cut(self.call(FINDER_WORLD, scripts::snapshot(), SCRIPT).await?.unwrap_or_default(), acts::MOST_SNAPSHOT))
    }

    async fn capture(&self, path: &str) -> bool {
        if !acts::png_path(path) {
            return false;
        }
        let rx = self.page.read_with(&self.cx, |page, _| page.png());
        let Some(Some(png)) = self.heard(rx, SCRIPT).await else { return false };
        let path = path.to_string();
        self.cx.background_executor().spawn(async move { std::fs::write(path, png).is_ok() }).await
    }

    async fn evaluate(&self, expression: &str) -> Result<Option<String>, String> {
        let answer = self.call(EVALUATE_WORLD, scripts::evaluate(expression), EXPRESSION).await?;
        Ok(scripts::evaluated(answer.as_deref())?.map(|result| cut(result, MOST_RESULT)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_is_cut_at_a_character_and_a_short_one_is_left_alone() {
        assert_eq!(cut("abc".into(), 10), "abc");
        assert_eq!(cut("abcdef".into(), 3), "abc");
        // "é" is two bytes: a cut in the middle of it goes back to the character before.
        assert_eq!(cut("aéb".into(), 2), "a");
        assert_eq!(cut("aéb".into(), 3), "aé");
    }
}
