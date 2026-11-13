//! Listen, as the Jobs screen holds it: the Jobs Fleet sends are compared with what was last seen, what is new is
//! handed to the `Listener`, and the thread draws its cards (`spoken.rs`). A walk's scenes change a stand-in for
//! the records and go through the same comparison.

use gpui::{App, Context};

use super::view_tabs::LiveJobView;
use super::JobView;
use crate::listen::{held, scenes};

impl JobView {
    /// Hand what is new since the last look to the Listener. Not listening costs a flag, and forgets what was seen.
    pub(super) fn listen_sync(&mut self, cx: &mut Context<Self>) {
        let prefs = held();
        if !prefs.listening && self.heard.is_idle() {
            return;
        }
        let said = match &self.mock_records {
            Some(records) => self.heard.fresh(records, &prefs),
            None => self.heard.fresh(self.list.read(cx).records(), &prefs),
        };
        if said.is_empty() {
            return;
        }
        self.listen.update(cx, |listener, cx| said.into_iter().for_each(|said| listener.say(said, cx)));
    }

    /// A card came or could not be spoken: the shown Job's list measures its tail again.
    pub(super) fn listen_changed(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self.shown.clone() {
            self.sync(&id, cx);
        }
        cx.notify();
    }

    /// A step of the Listen walk: Fleet's side changes as the scene says, and the same comparison reads it.
    pub(super) fn listen_scene(&mut self, name: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let fixture = scenes::fixture()?;
        let scene = fixture.scenes.get(name).ok_or_else(|| format!("no Listen scene named {name}"))?;
        if let Some(preferences) = &scene.preferences {
            crate::listen::hold(preferences);
        }
        // The Jobs as they stand are what the scene changes, and are looked at first so only the change is new.
        self.listen_sync(cx);
        let mut records = self.mock_records.take().unwrap_or_else(|| self.list.read(cx).records().to_vec());
        for change in &scene.jobs {
            let record = records.iter_mut().find(|record| record.id.as_str() == change.id).ok_or_else(|| format!("no Job {} in the scene {name}", change.id))?;
            if let Some(waiting) = &change.waiting_for {
                record.waiting_for = waiting.clone();
            }
            if let Some(done) = &change.done_since {
                record.done_since = Some(done.clone());
            }
            if let Some(summary) = &change.done_summary {
                record.done_summary = Some(summary.clone());
            }
        }
        self.mock_records = Some(records);
        self.listen.update(cx, |listener, cx| {
            listener.mock(|mock| {
                mock.missing = scene.missing.unwrap_or(mock.missing);
                mock.ahead = std::time::Duration::from_secs_f64(scene.ahead_secs.unwrap_or(0.));
            });
            if scene.finish {
                listener.finish_now(cx);
            }
        });
        self.listen_sync(cx);
        cx.notify();
        Ok(())
    }
}

/// The walk's `Listen` step, on the Jobs screen that is drawn.
pub fn scene(name: &str, cx: &mut App) -> Result<(), String> {
    let view = cx.try_global::<LiveJobView>().and_then(|live| live.0.upgrade()).ok_or("no Jobs screen is drawn")?;
    view.update(cx, |view, cx| view.listen_scene(name, cx))
}
