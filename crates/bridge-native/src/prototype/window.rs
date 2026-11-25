//! The pop-out: the prototype in a window of its own, with the note capture over it.
//!
//! **One capture, two homes** (`capture.rs`): the annotation layer is mounted here as the Jobs
//! screen mounts it, so ⌥⌘A works over the scene and a note names the part it was made on. The
//! notes go to the Job the prototype belongs to, through `annotate::live::deliver`, as the layer's
//! own send does; this window has no other Job to send to and starts none.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{
    canvas, deferred, div, prelude::*, App, Bounds, Context, Entity, FocusHandle, Focusable, Pixels, Render, SharedString, Subscription,
    TitlebarOptions, WeakEntity, Window, WindowOptions,
};

use super::{Home, PrototypeView, Source};
use crate::annotate::{self, Asked, Layer, Shoot, Toggle};
use crate::dispatch::face::Offered;
use crate::fleet::FleetConnection;
use crate::labels::label_root;
use crate::look::{token, ActiveLook};

pub struct PrototypeWindow {
    view: Entity<PrototypeView>,
    layer: Entity<Layer>,
    area: Rc<Cell<Option<Bounds<Pixels>>>>,
    job: Offered,
    fleet: Option<Entity<FleetConnection>>,
    focus: FocusHandle,
    _subs: Vec<Subscription>,
}

/// Open the prototype in a window of its own. Opening it again from the same tab makes another.
pub fn open(source: Source, job: Offered, fleet: Option<Entity<FleetConnection>>, cx: &mut App) -> Option<WeakEntity<PrototypeWindow>> {
    let title = format!("{} prototype", job.title);
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions { title: Some(SharedString::from(title)), ..Default::default() }),
        window_bounds: Some(gpui::WindowBounds::Windowed(Bounds::centered(None, gpui::size(gpui::px(960.), gpui::px(720.)), cx))),
        ..Default::default()
    };
    let held: Rc<RefCell<Option<WeakEntity<PrototypeWindow>>>> = Rc::default();
    let keep = Rc::clone(&held);
    let opened = cx.open_window(options, move |window, cx| {
        let opened = cx.new(|cx| PrototypeWindow::new(source, job, fleet, window, cx));
        let focus = opened.read(cx).focus.clone();
        focus.focus(window, cx);
        *keep.borrow_mut() = Some(opened.downgrade());
        opened
    });
    opened.ok()?;
    let weak = held.borrow_mut().take();
    weak
}

impl PrototypeWindow {
    pub fn new(source: Source, job: Offered, fleet: Option<Entity<FleetConnection>>, window: &mut Window, cx: &mut Context<Self>) -> PrototypeWindow {
        let area: Rc<Cell<Option<Bounds<Pixels>>>> = Rc::default();
        let view = cx.new(|cx| {
            let mut view = PrototypeView::new(source, Home::Window, cx);
            view.set_job(job.id.clone());
            view
        });
        let layer = cx.new(|cx| {
            let mut layer = Layer::new(Rc::clone(&area), vec![job.clone()], cx);
            layer.screen = Some("Prototype".into());
            layer
        });
        if let Some(dir) = annotate::store::dir() {
            layer.update(cx, |layer, cx| layer.keep_in(dir, window, cx));
        }
        let subs = vec![
            cx.observe(&layer, |_, _, cx| cx.notify()),
            cx.observe(&view, |_, _, cx| cx.notify()),
            cx.subscribe_in(&layer, window, |this, layer, asked: &Asked, _, cx| this.send_notes(layer.clone(), asked, cx)),
        ];
        PrototypeWindow { view, layer, area, job, fleet, focus: cx.focus_handle(), _subs: subs }
    }

    pub(super) fn view(&self) -> &Entity<PrototypeView> {
        &self.view
    }

    pub(super) fn layer(&self) -> &Entity<Layer> {
        &self.layer
    }

    /// The prototype saved again while the window is up.
    pub fn set_source(&mut self, source: Source, cx: &mut Context<Self>) {
        self.view.update(cx, |view, cx| view.set_source(source, cx));
    }

    /// The notes go to this Job. A new Job is not started from here.
    fn send_notes(&mut self, layer: Entity<Layer>, asked: &Asked, cx: &mut Context<Self>) {
        let Asked::Send { to, .. } = asked;
        let title = self.job.title.to_string();
        if to.is_none() {
            layer.update(cx, |layer, cx| layer.refused("a new Job", "A prototype window sends notes to its own Job", cx));
            return;
        }
        let Some(port) = self.fleet.as_ref().and_then(|fleet| fleet.read(cx).held().port()) else {
            layer.update(cx, |layer, cx| layer.refused(&title, "Fleet is not connected", cx));
            return;
        };
        let look = cx.look();
        let bytes = |name| {
            let rgba = look.color(name).to_rgb();
            [rgba.r, rgba.g, rgba.b, rgba.a].map(|part| (part * 255.).round() as u8)
        };
        let ink = annotate::live::Ink(bytes(token::ACCENT), bytes(token::TEXT_ON_ACCENT));
        let outgoing = layer.read(cx).outgoing();
        let job = self.job.id.to_string();
        let task = cx.background_spawn(async move { annotate::live::deliver(port, &job, outgoing, ink) });
        let session = self.job.id.clone();
        cx.spawn(async move |_, cx| {
            let answer = task.await;
            let _ = cx.update(|cx| match answer {
                Ok(()) => layer.update(cx, |layer, cx| {
                    layer.sending_to(session.clone());
                    layer.sent(&title, false, cx);
                }),
                Err(why) => {
                    layer.update(cx, |layer, cx| layer.refused(&title, &why.to_string(), cx));
                    why.tell(format!("Notes not sent to \"{title}\""), cx);
                }
            });
        })
        .detach();
    }
}

impl Focusable for PrototypeWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PrototypeWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let area = Rc::clone(&self.area);
        let text = cx.look().color(token::FG_DEFAULT);
        label_root(
            div()
                .id("prototype-window")
                .key_context("prototype")
                .track_focus(&self.focus)
                .on_action(cx.listener(|this, _: &Toggle, window, cx| this.layer.update(cx, |layer, cx| layer.toggle(window, cx))))
                .on_action(cx.listener(|this, _: &Shoot, window, cx| this.layer.update(cx, |layer, cx| layer.shoot(window, cx))))
                .on_key_down(cx.listener(|_, event: &gpui::KeyDownEvent, window, cx| {
                    if crate::keymap::traverse(event, window, cx) {
                        cx.stop_propagation();
                    }
                }))
                .relative()
                .size_full()
                .text_color(text)
                .child(self.view.clone())
                .child(canvas(move |bounds, _, _| area.set(Some(bounds)), |_, _, _, _| {}).absolute().top_0().left_0().size_full())
                .child(deferred(self.layer.clone()).with_priority(annotate::OVER)),
        )
    }
}
