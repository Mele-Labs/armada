//! A mod's panel with no Job beside it, for a surface that hosts it as Classic's shared panel is hosted
//! (`shared-panel/mod.tsx`): Settings' mod rows open it over any screen. Its body is the one the Job
//! view draws; its presses go to Fleet itself, and a press that opens a Job asks the Job view to.

use std::rc::Rc;

use gpui::{div, prelude::*, App, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight, Role, SharedString, Subscription, Task, Window};
use ipc::{ModKind, ModSummary};

use super::mods_made::{made_title, panel_body};
use super::rows::{glyph, tip, Size};
use crate::fleet::FleetConnection;
use crate::labels::labelled;
use crate::look::{token, ActiveLook};

/// What the panel asks of the surface that hosts it.
#[derive(Clone, Debug, PartialEq)]
pub enum ModPanelAsked {
    /// Its close was pressed, or Esc: the host takes it away.
    Close,
    /// A Job opened from it, Change it's or the one that made the mod: the host gets out of the way.
    OpenedJob,
}

/// Opens a Job in the Job view, with words kept in its message box where there are some. Set by the
/// Job view.
#[derive(Clone)]
pub struct OpenJobFrom(pub Rc<dyn Fn(String, Option<String>, &mut App)>, pub Rc<dyn Fn(&str, &App) -> Option<String>>);

impl gpui::Global for OpenJobFrom {}

pub struct ModPanel {
    name: String,
    fleet: Option<Entity<FleetConnection>>,
    /// What he pressed and what Fleet's list said then, until the list says otherwise.
    switched: Option<(bool, bool)>,
    focus: FocusHandle,
    _watch: Option<Subscription>,
}

impl EventEmitter<ModPanelAsked> for ModPanel {}

impl Focusable for ModPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl ModPanel {
    /// The panel on mod `name`, by its folder name as Fleet lists it. With no Fleet, the walk's mods.
    pub fn new(name: impl Into<String>, fleet: Option<Entity<FleetConnection>>, cx: &mut Context<Self>) -> Self {
        let watch = fleet.as_ref().map(|fleet| cx.observe(fleet, |_, _, cx| cx.notify()));
        ModPanel { name: name.into(), fleet, switched: None, focus: cx.focus_handle(), _watch: watch }
    }

    /// Whether Fleet lists a mod by that name, so a host never opens a panel on nothing.
    pub fn lists(name: &str, fleet: Option<&Entity<FleetConnection>>, cx: &App) -> bool {
        Self::list_of(fleet, cx).iter().any(|one| one.name == name)
    }

    fn list_of(fleet: Option<&Entity<FleetConnection>>, cx: &App) -> Vec<ModSummary> {
        match fleet {
            Some(fleet) => fleet.read(cx).held().mods.clone().unwrap_or_default(),
            None => crate::mock::mods(),
        }
    }

    fn row(&self, cx: &App) -> Option<ModSummary> {
        Self::list_of(self.fleet.as_ref(), cx).into_iter().find(|one| one.name == self.name)
    }

    fn on(&self, row: &ModSummary) -> bool {
        match self.switched {
            Some((was, now)) if was == row.enabled => now,
            _ => row.enabled,
        }
    }

    /// The switch moves at once; a refusal puts it back and says why, as the Job view's does.
    fn set_enabled(&mut self, name: String, on: bool, cx: &mut Context<Self>) {
        let Some(row) = self.row(cx) else { return };
        self.switched = Some((row.enabled, on));
        cx.notify();
        let Some(port) = self.port(cx) else { return };
        let body = ipc::encode(&ipc::SetModEnabled { name, enabled: on });
        let sent = posted(port, "/mods/enable", body, cx);
        cx.spawn(async move |this, cx| {
            if let Err(why) = sent.await {
                let _ = this.update(cx, |this, cx| {
                    this.switched = None;
                    cx.notify();
                    cx.defer(move |cx| crate::toasts::tell(why, cx));
                });
            }
        })
        .detach();
    }

    /// Change it: an empty Mod Job aimed at what the mod is, opened with its words in the box.
    fn change(&mut self, title: String, kind: ModKind, cx: &mut Context<Self>) {
        let Some(port) = self.port(cx) else {
            return crate::toasts::tell("Mod Job not started: Fleet is not connected", cx);
        };
        let start = super::mods_made::start_of(kind);
        let words = super::mods_made::change_words(&title, kind);
        let sent = posted(port, "/mods/session", ipc::encode(&start), cx);
        cx.spawn(async move |this, cx| {
            let started = sent.await.and_then(|bytes| ipc::decode::<ipc::ModSessionStarted>("ModSessionStarted", &bytes).map_err(|why| why.to_string()));
            let _ = this.update(cx, |this, cx| match started {
                Ok(started) => this.open_job(started.session_id.as_str().to_string(), Some(words), cx),
                Err(why) => cx.defer(move |cx| crate::toasts::tell(format!("Mod Job not started: {why}"), cx)),
            });
        })
        .detach();
    }

    fn open_job(&mut self, id: String, words: Option<String>, cx: &mut Context<Self>) {
        if let Some(open) = cx.try_global::<OpenJobFrom>().cloned() {
            cx.defer(move |cx| (open.0)(id, words, cx));
        }
        cx.emit(ModPanelAsked::OpenedJob);
    }

    fn port(&self, cx: &App) -> Option<u16> {
        self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port())
    }
}

/// A POST to Fleet off the main thread: its answer's bytes, or Fleet's why.
fn posted(port: u16, path: &'static str, body: Result<String, ipc::Unencodable>, cx: &App) -> Task<Result<Vec<u8>, String>> {
    cx.background_spawn(async move {
        let body = body.map_err(|why| format!("{why:?}"))?;
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|why| why.to_string())?;
        let (status, bytes) = runtime.block_on(crate::transport::post(port, path, body))?;
        if (200..300).contains(&status) {
            Ok(bytes)
        } else {
            Err(ipc::decode::<ipc::WireError>("WireError", &bytes).map_or_else(|_| format!("Fleet answered {status}"), |error| error.message))
        }
    })
}

impl Render for ModPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        let row = self.row(cx);
        let kind = row.as_ref().and_then(|one| one.kind).unwrap_or(ModKind::Theme);
        let title = made_title(&self.name, kind);
        let on = row.as_ref().map(|one| self.on(one));
        let maker = row.as_ref().and_then(|one| one.made_by.clone()).map(|maker| {
            let id = maker.as_str().to_string();
            let named = cx.try_global::<OpenJobFrom>().and_then(|hook| (hook.1)(&id, cx)).unwrap_or_else(|| "Job".to_string());
            (id, named)
        });
        let body = panel_body(
            row.as_ref(),
            &self.name,
            &title,
            kind,
            on,
            maker,
            &look,
            cx,
            |this: &mut Self, name, on, _, cx| this.set_enabled(name, on, cx),
            |this: &mut Self, title, kind, _, cx| this.change(title, kind, cx),
            |this: &mut Self, id, _, cx| this.open_job(id, None, cx),
        );
        let close = labelled(
            div()
                .id("mod-panel-close")
                .flex_none()
                .p(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_SM))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .tooltip(tip("Close  Esc"))
                .child(glyph(&look, "x", Size::Small, look.color(token::FG_MUTED)))
                .on_click(cx.listener(|_, _, _, cx| cx.emit(ModPanelAsked::Close))),
            Role::Button,
            "Close",
        );
        let head = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_6))
            .py(look.length(token::SPACE_3))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .child(div().flex_1().min_w_0().truncate().text_size(look.length(token::TEXT_LG)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child(SharedString::from(title.clone())))
            .child(close);
        labelled(
            div()
                .id("mod-panel")
                .track_focus(&self.focus)
                .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, _, cx| {
                    if event.keystroke.key == "escape" {
                        cx.stop_propagation();
                        cx.emit(ModPanelAsked::Close);
                    }
                }))
                .size_full()
                .flex()
                .flex_col()
                .bg(look.color(token::BG_OVERLAY))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(head)
                .child(body),
            Role::Region,
            title,
        )
    }
}
