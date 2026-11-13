//! The preferences Settings › Layout and Settings › Time write (`layout_choices`, `time_format`). The connection keeps
//! neither, so the pages read `GET /preferences` each time Settings opens on one, and a save's answer replaces it.
//! **A change is drawn at once** (`fleet-layout.ts`, `fleet-time-format.ts`): a refusal puts back what was read and
//! says why.

use std::collections::BTreeMap;
use std::time::Duration;

use gpui::{App, Context, FocusHandle, Task, Window};

use super::entries::Body;
use super::overlay::SettingsOverlay;
use super::wire::Put;

/// How often the Time page's sample is drawn again.
const TICK: Duration = Duration::from_secs(1);

pub struct Prefs {
    pub read: Option<Result<ipc::Preferences, String>>,
    asked: bool,
    /// Saves not yet answered: an answer is drawn only once the last of them is in.
    pending: usize,
    /// What a refused save said, by preference.
    pub refused: BTreeMap<&'static str, String>,
    /// Each row's keys, by `layout:<region>:<id>`, `layout-reset` or `time:<switch>`.
    pub keys: BTreeMap<String, FocusHandle>,
    _read: Option<Task<()>>,
    _tick: Option<Task<()>>,
}

impl Prefs {
    pub fn new() -> Prefs {
        Prefs { read: None, asked: false, pending: 0, refused: BTreeMap::new(), keys: BTreeMap::new(), _read: None, _tick: None }
    }

    /// Settings opened again: read afresh.
    pub fn reopen(&mut self) {
        self.asked = false;
        self.refused.clear();
    }

    pub fn held(&self) -> Option<&ipc::Preferences> {
        self.read.as_ref().and_then(|read| read.as_ref().ok())
    }
}

impl SettingsOverlay {
    fn prefs_shown(&self, cx: &App) -> Option<Body> {
        self.open.then(|| self.shown(cx).map(|one| one.body)).flatten().filter(|body| matches!(body, Body::Layout | Body::Time | Body::Ways))
    }

    /// Read once each time Settings opens on Layout or Time, and keep Time's sample moving while it is drawn.
    pub(super) fn read_prefs(&mut self, cx: &mut Context<Self>) {
        let Some(body) = self.prefs_shown(cx) else { return };
        if body == Body::Time && self.prefs._tick.is_none() {
            self.prefs._tick = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(TICK).await;
                let going = this.update(cx, |this, cx| {
                    let going = this.prefs_shown(cx) == Some(Body::Time);
                    if going {
                        cx.notify();
                    } else {
                        this.prefs._tick = None;
                    }
                    going
                });
                if !matches!(going, Ok(true)) {
                    break;
                }
            }));
        }
        if self.prefs.asked {
            return;
        }
        self.prefs.asked = true;
        let read = self.wire.preferences(cx);
        self.prefs._read = Some(cx.spawn(async move |this, cx| {
            let answer = read.await;
            let _ = this.update(cx, |this, cx| {
                if this.prefs.pending == 0 {
                    this.prefs.read = Some(answer.map_err(|why| why.said));
                }
                cx.notify();
            });
        }));
    }

    /// Keys for each row, made before they are drawn.
    pub(super) fn prefs_keys(&mut self, cx: &mut Context<Self>) {
        let mut names: Vec<String> = super::layout::REGIONS.iter().flat_map(|region| region.rows.iter().filter(|row| row.hideable).map(move |row| format!("layout:{}:{}", region.id, row.id))).collect();
        names.push("layout-reset".into());
        names.extend(super::time::SWITCHES.iter().map(|(name, _)| format!("time:{name}")));
        names.extend(super::ways::key_names());
        for name in names {
            self.prefs.keys.entry(name).or_insert_with(|| cx.focus_handle().tab_stop(true));
        }
    }

    /// `POST /preferences/save` with one preference's text, drawn before Fleet answers.
    pub(super) fn save_pref(&mut self, name: &'static str, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.save_put_pref(name, Put::Text(text), window, cx);
    }

    /// One preference saved as the word or the switch it is, drawn before Fleet answers and put back, with Fleet's
    /// reason, if Fleet refuses.
    pub(super) fn save_put_pref(&mut self, name: &'static str, put: Put, window: &mut Window, cx: &mut Context<Self>) {
        let Some(was) = self.prefs.held().cloned() else { return };
        if !self.live {
            return;
        }
        let mut now = was.clone();
        if !super::ways::apply(&mut now, name, &put) {
            return;
        }
        self.prefs.read = Some(Ok(now));
        self.prefs.refused.remove(name);
        self.prefs.pending += 1;
        let sent = self.wire.save_put(name, put, cx);
        cx.spawn_in(window, async move |this, cx| {
            let answer = sent.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.prefs.pending -= 1;
                match answer {
                    Ok(preferences) => {
                        if this.prefs.pending == 0 {
                            this.prefs.read = Some(Ok(preferences.clone()));
                        }
                        this.wire.preferences_saved(preferences, window, cx);
                    }
                    Err(why) => {
                        if !this.open {
                            let said = why.said.clone();
                            cx.defer(move |cx| crate::toasts::tell(said, cx));
                        }
                        this.prefs.read = Some(Ok(was));
                        this.prefs.refused.insert(name, why.said);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}
