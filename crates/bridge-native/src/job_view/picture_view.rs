//! A picture opened in a view that fills the app below the title bar, not the screen: a press on a take, a drawing part, a thread's
//! picture. It opens at fit and zooms with the one helper (`zoomer.rs`). A take's head names the take and its two neighbours, ← and →
//! step between them, and a Pick button sits in the head; a part that asks keeps its Approve and Ask for changes, or Send pick, at
//! the foot. Esc closes it and the keys go back to where they were. A press on an artifact card grows the view out of the card, and
//! on a drawing a right press leaves a note on the part under it (`sketch_notes.rs`).

use std::rc::Rc;
use std::sync::Arc;

use gpui::{anchored, deferred, div, img, prelude::*, px, size, Anchor, Animation, AnimationExt, AnyElement, App, Bounds, Context, FocusHandle, Image, KeyDownEvent, MouseButton, MouseDownEvent, ObjectFit, Pixels, Role, SharedString, Size, Window};
use ipc::{PartContent, Scene};

use super::parts_card::pick_mark;
use super::rows::{glyph, tip, Size as GlyphSize};
use super::sketch_card::{act, Kind};
use super::zoomer::{self, Stage, Zoomer, Zooms};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::scene::{Clock, Fit, Lens};

/// What the view holds.
pub(super) enum What {
    /// A file a message carried: its picture and its size, under its name.
    Raster { image: Arc<Image>, natural: Size<f32>, name: SharedString },
    /// A take of a takes part, by the part's key and the take's place.
    Take { key: String, at: usize },
    /// A drawing part, by the part's key.
    Drawing { key: String },
}

/// The picture open, how far it is zoomed and moved, and where the keys were before.
pub(super) struct Viewing {
    what: What,
    focus: FocusHandle,
    back: Option<FocusHandle>,
    zoom: Zoomer,
    /// The card it grew out of, and which open this is.
    from: Option<(Bounds<Pixels>, usize)>,
    /// On a drawing: the part a note would be about, and the note being written.
    pub(super) target: Option<super::sketch_notes::Part>,
    pub(super) writing: Option<super::sketch_notes::Writing>,
}

/// The app's area below the title bar: where the view stands.
pub(super) fn app_area(window: &Window, cx: &App) -> Bounds<Pixels> {
    let room = crate::room::of(window, cx);
    let bar = cx.look().length(token::H_TITLE_BAR);
    Bounds::new(room.origin + gpui::point(px(0.), bar), size(room.size.width, (room.size.height - bar).max(px(0.))))
}

/// The letter a take's place is named by.
fn letter(at: usize) -> char {
    char::from(b'A' + u8::try_from(at % 26).unwrap_or(0))
}

/// The take `by` places on from `at` among `len`, round the ends.
pub(super) fn stepped(at: usize, by: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (at as isize + by).rem_euclid(len as isize) as usize
}

/// The words under a take: its place among all of them, as a quiet line.
pub(super) fn take_of(at: usize, len: usize) -> String {
    format!("Take {} of {len}", letter(at))
}

impl JobView {
    /// Whether the view over the app answers the part of `key`: over the review sheet, or once its artifact no longer asks. From
    /// the artifact's own tab it is read only, as the tab is.
    fn answers_in_view(&self, key: &str, cx: &App) -> bool {
        self.review_open() || super::parts_card::split_key(key).is_none_or(|(artifact, _)| !self.reads_only(artifact, cx))
    }

    pub(super) fn open_picture(&mut self, image: Arc<Image>, natural: Size<f32>, name: SharedString, window: &mut Window, cx: &mut Context<Self>) {
        self.open_viewing(What::Raster { image, natural, name }, window, cx);
    }

    /// A take of a takes part, read large: its head names it and its neighbours.
    pub(super) fn open_take(&mut self, key: &str, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.open_viewing(What::Take { key: key.to_string(), at }, window, cx);
    }

    /// A drawing part, read large.
    pub(super) fn open_drawing(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.open_viewing(What::Drawing { key: key.to_string() }, window, cx);
    }

    fn open_viewing(&mut self, what: What, window: &mut Window, cx: &mut Context<Self>) {
        let focus = cx.focus_handle();
        let back = window.focused(cx);
        window.focus(&focus, cx);
        self.pictures.viewing = Some(Viewing { what, focus, back, zoom: Zoomer::default(), from: None, target: None, writing: None });
        cx.notify();
    }

    /// A press on artifact `id`'s card: its first drawing grows out of the card to fill the app, with every act the card has at
    /// its foot. A card with no drawing opens in the review panel.
    pub(super) fn open_card(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(job) = self.shown.clone() else { return };
        let drawing = self.artifact_of(&job, id).and_then(|artifact| artifact.parts.iter().find(|part| matches!(part.content, PartContent::Drawing { .. })).map(|part| part.id.clone()));
        let Some(part) = drawing else {
            self.open_artifact_review(id, window, cx);
            return;
        };
        self.grow_drawing(&super::parts_card::part_key(id, &part), window, cx);
    }

    /// Drawing part `key` grows out of its artifact's card to fill the app.
    pub(super) fn grow_drawing(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) {
        let from = super::parts_card::split_key(key).and_then(|(artifact, _)| self.pictures.cards.borrow().get(artifact).copied());
        self.open_viewing(What::Drawing { key: key.to_string() }, window, cx);
        self.pictures.opens += 1;
        let opens = self.pictures.opens;
        if let Some(viewing) = self.pictures.viewing.as_mut() {
            viewing.from = from.map(|bounds| (bounds, opens));
        }
    }

    /// The drawing part the view holds, where it holds one: its key, the key it is drawn under, and the view's focus.
    pub(super) fn viewed_drawing(&self) -> Option<(&str, String, FocusHandle)> {
        let viewing = self.pictures.viewing.as_ref()?;
        let What::Drawing { key } = &viewing.what else { return None };
        Some((key, self.held_of(&viewing.what, "view")?, viewing.focus.clone()))
    }

    /// Esc or Close: it shuts and the keys go back to where they were.
    pub(super) fn close_picture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(viewing) = self.pictures.viewing.take() {
            if let Some(back) = viewing.back {
                window.focus(&back, cx);
            }
        }
        cx.notify();
    }

    /// ← and → step a take to the one before and after it, and the new one opens at fit.
    pub(super) fn step_take(&mut self, by: isize, cx: &mut Context<Self>) {
        let Some(What::Take { key, at }) = self.pictures.viewing.as_ref().map(|viewing| &viewing.what) else { return };
        let (key, at) = (key.clone(), *at);
        let Some((takes, _, _)) = self.takes_of_key(&key) else { return };
        if let Some(viewing) = self.pictures.viewing.as_mut() {
            if let What::Take { at: now, .. } = &mut viewing.what {
                *now = stepped(at, by, takes.len());
                if *now != at {
                    crate::ui_sounds::moved();
                }
            }
            viewing.zoom.fit();
        }
        cx.notify();
    }

    /// The scene the view draws and the key it is kept by, where it holds a drawing rather than a file.
    fn scene_of(&self, what: &What) -> Option<(String, Rc<Scene>)> {
        self.scene_of_as(what, "view")
    }

    /// The same, held under `prefix`: the review panel's detail draws a drawing beside the view that enlarges it, and the two
    /// must not share one canvas.
    pub(super) fn scene_of_as(&self, what: &What, prefix: &str) -> Option<(String, Rc<Scene>)> {
        let (held, text) = self.held_text(what, prefix)?;
        let raw: ipc::RawScene = serde_json::from_str(&text).ok()?;
        let scene = crate::scene::read(&held, &raw)?;
        Some((held, scene))
    }

    /// The key a drawing is held under, without reading it.
    fn held_of(&self, what: &What, prefix: &str) -> Option<String> {
        self.held_text(what, prefix).map(|(held, _)| held)
    }

    fn held_text(&self, what: &What, prefix: &str) -> Option<(String, String)> {
        let job = self.shown.as_ref()?;
        let (key, text, tag) = match what {
            What::Raster { .. } => return None,
            What::Take { key, at } => {
                let (takes, _, _) = self.takes_of_key(key)?;
                let text = takes.get(*at)?.scene.clone()?;
                let tag = format!("{at}:{}", text.len());
                (key.clone(), text, tag)
            }
            What::Drawing { key } => {
                let (_, part) = self.part_of_key(key)?;
                let PartContent::Drawing { scene } = part.content else { return None };
                let tag = format!("{}:{}", part.drawn, scene.len());
                (key.clone(), scene, tag)
            }
        };
        Some((format!("{prefix}:{job}:{key}:{tag}"), text))
    }

    /// The key of the drawing part the viewer holds, where it holds one.
    #[cfg(test)]
    pub(super) fn viewing_drawing(&self) -> Option<&str> {
        match self.pictures.viewing.as_ref().map(|viewing| &viewing.what) {
            Some(What::Drawing { key }) => Some(key),
            _ => None,
        }
    }

    /// The viewer's keys: Esc closes, ← and → step a take, and the zoom's own. A text box keeps its keys.
    fn picture_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if window.context_stack().iter().any(|context| context.contains(crate::keymap::TEXT_BOX)) {
            return;
        }
        let key = &event.keystroke;
        let plain = !key.modifiers.modified();
        let take = self.pictures.viewing.as_ref().is_some_and(|viewing| matches!(viewing.what, What::Take { .. }));
        let drawing = self.viewed_drawing().map(|(key, _, _)| key.to_string());
        match key.key.as_str() {
            "escape" if plain => self.close_picture(window, cx),
            "left" if plain && take => self.step_take(-1, cx),
            "right" if plain && take => self.step_take(1, cx),
            "enter" if drawing.is_some() && key.modifiers.platform && !self.held_notes().is_empty() => self.send_sketch_notes(window, cx),
            "f10" if drawing.is_some() && key.modifiers.shift => self.note_menu_by_keys(window, cx),
            "tab" if drawing.is_some() && !key.modifiers.platform => self.walk_parts(key.modifiers.shift, cx),
            _ if zoomer::keyed(self, key, window, cx) => {}
            // A drawing's view answers as its card does: A, C, D and the take keys.
            _ => match drawing.as_deref().and_then(super::parts_card::split_key).map(|(artifact, _)| artifact.to_string()) {
                Some(artifact) => return self.parts_key(&artifact, event, window, cx),
                None => return,
            },
        }
        cx.stop_propagation();
    }

    /// The title the head reads: a take's own name, a part's artifact, or the file's.
    fn picture_name(&self, what: &What) -> SharedString {
        match what {
            What::Raster { name, .. } => name.clone(),
            What::Take { key, at } => self.takes_of_key(key).and_then(|(takes, _, _)| takes.get(*at).map(|take| SharedString::from(pick_mark(&take.title).0.to_string()))).unwrap_or_default(),
            What::Drawing { key } => {
                let title = self.part_of_key(key).and_then(|(artifact, _)| self.artifact_of(self.shown.as_deref()?, &artifact).map(|one| one.title.clone()));
                SharedString::from(title.unwrap_or_else(|| "Drawing".to_string()))
            }
        }
    }

    /// A take's neighbour, named in a column of its own: its arrow and its name, and the arrow key that steps to it.
    fn neighbour(&self, look: &Look, to: usize, before: bool, name: &str, cx: &Context<Self>) -> AnyElement {
        let said = if before { format!("\u{2039} {name}") } else { format!("{name} \u{203a}") };
        let held = SharedString::from(name.to_string());
        labelled(
            div()
                .id(SharedString::from(format!("picture-step:{to}")))
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .when(!before, |one| one.justify_end())
                .h(look.length(token::H_CONTROL))
                .px(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_MUTED))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                .tooltip(tip(held.clone()))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().min_w_0().truncate().child(SharedString::from(said)))
                .child(super::acts::kbd(look, if before { "\u{2190}" } else { "\u{2192}" }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(viewing) = this.pictures.viewing.as_mut() {
                        if let What::Take { at, .. } = &mut viewing.what {
                            *at = to;
                            viewing.zoom.fit();
                            cx.notify();
                        }
                    }
                })),
            Role::Button,
            held,
        )
        .into_any_element()
    }

    /// The head: a take's own name between its two neighbours, with Take B of 3 quiet beside it, or the title alone; Pick where a pick
    /// is asked; Close.
    fn picture_head(&self, viewing: &Viewing, look: &Look, cx: &Context<Self>) -> AnyElement {
        let name = self.picture_name(&viewing.what);
        let title = labelled(
            div()
                .id("picture-title")
                .min_w_0()
                .truncate()
                .text_size(look.length(token::TEXT_SM))
                .font_weight(gpui::FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_DEFAULT))
                .tooltip(tip(name.clone()))
                .child(name.clone()),
            Role::Heading,
            name.clone(),
        );
        let mut head = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .h(look.length(token::H_ROW) * 1.25)
            .px(look.length(token::SPACE_4))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_RAISED));
        let mut pick = None;
        if let What::Take { key, at } = &viewing.what {
            if let Some((takes, open, chosen)) = self.takes_of_key(key) {
                let open = open && self.answers_in_view(key, cx);
                let named = |to: usize| pick_mark(&takes[to].title).0.to_string();
                let (before, after) = (stepped(*at, -1, takes.len()), stepped(*at, 1, takes.len()));
                let (_, marked) = pick_mark(&takes[*at].title);
                head = head
                    .child(if takes.len() > 1 { self.neighbour(look, before, true, &named(before), cx) } else { div().flex_1().into_any_element() })
                    .child(div().flex().flex_none().items_baseline().gap(look.length(token::SPACE_2)).max_w(px(560.)).child(title).child(
                        div().flex_none().text_size(look.length(token::TEXT_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(if marked { format!("{}, Job's pick", take_of(*at, takes.len())) } else { take_of(*at, takes.len()) })),
                    ))
                    .child(if takes.len() > 1 { self.neighbour(look, after, false, &named(after), cx) } else { div().flex_1().into_any_element() });
                let on = chosen == Some(*at);
                let (press, at) = (key.clone(), *at);
                pick = (open || on).then(|| {
                    let button = act(look, "picture-pick".to_string(), if on { "Picked" } else { "Pick" }, if on { Kind::Primary } else { Kind::Secondary }, "", open)
                        .when(on, |button| button.child(glyph(look, "check", GlyphSize::Small, look.color(token::TEXT_ON_ACCENT))))
                        .tooltip(tip(if on { "This take is your pick" } else { "Make this take your pick" }));
                    let button = if open { button.on_click(cx.listener(move |this, _, _, cx| this.choose_take(&press, at, cx))) } else { button };
                    labelled(button, Role::Button, if on { "Picked" } else { "Pick" })
                });
            }
        } else {
            head = head.child(div().flex_1().min_w_0().child(title));
        }
        let close = labelled(
            div()
                .id("picture-close")
                .flex_none()
                .size(look.length(token::H_CONTROL))
                .flex()
                .items_center()
                .justify_center()
                .rounded(look.length(token::RADIUS_MD))
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph(look, "x", GlyphSize::Small, look.color(token::FG_DEFAULT)))
                .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new("Close").key_binding(gpui::Keystroke::parse("escape").ok().map(gpui_component::kbd::Kbd::new)).build(window, cx))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, window, cx| this.close_picture(window, cx))),
            Role::Button,
            "Close",
        );
        head.children(pick).child(close).into_any_element()
    }

    /// The picture open over the app, below the title bar.
    pub(super) fn render_picture(&self, window: &Window, cx: &Context<Self>) -> Option<impl IntoElement> {
        let viewing = self.pictures.viewing.as_ref()?;
        let look = cx.look();
        let area = app_area(window, cx);
        let scene = self.scene_of(&viewing.what);
        let head = self.picture_head(viewing, look, cx);
        let bar = |acts: AnyElement| div().flex_none().px(look.length(token::SPACE_4)).py(look.length(token::SPACE_3)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_DEFAULT)).bg(look.color(token::BG_RAISED)).child(acts);
        let foot = match &viewing.what {
            What::Take { key, .. } | What::Drawing { key } if !self.answers_in_view(key, cx) => None,
            What::Take { key, .. } =>self.part_acts(key, look, cx).map(bar).map(IntoElement::into_any_element),
            // A drawing keeps everything its card asks: its own Approve and Ask for changes, the pick, and Dismiss.
            What::Drawing { key } => {
                let acts = self.part_acts(key, look, cx).map(bar);
                let card = super::parts_card::split_key(key)
                    .and_then(|(artifact, _)| self.pages.get(self.shown.as_ref()?).and_then(|page| self.parts_pinned_footer(super::acts::SHEET, artifact, page, look, cx)))
                    .map(|card| div().flex_none().bg(look.color(token::BG_RAISED)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(card));
                (acts.is_some() || card.is_some()).then(|| div().flex_none().flex().flex_col().children(acts).children(card).into_any_element())
            }
            What::Raster { .. } => None,
        };
        // The first frame stands in the area's own size; the probe in the body gives the view's from then on.
        let view = viewing.zoom.view().unwrap_or(size(f32::from(area.size.width), f32::from(area.size.height)));
        let stage = self.stage_of(viewing, view, cx);
        let readout = stage.map(|stage| viewing.zoom.readout(stage));
        let content = match (&viewing.what, scene) {
            (What::Raster { image, .. }, _) => stage.map(|stage| {
                let shown = size(stage.fit.width * viewing.zoom.scale, stage.fit.height * viewing.zoom.scale);
                div()
                    .absolute()
                    .left(px(view.width / 2. - shown.width / 2. + viewing.zoom.pan.x))
                    .top(px(view.height / 2. - shown.height / 2. + viewing.zoom.pan.y))
                    .w(px(shown.width))
                    .h(px(shown.height))
                    .child(img(image.clone()).size_full().object_fit(ObjectFit::Contain))
                    .into_any_element()
            }),
            (What::Drawing { .. }, Some((held, scene))) => {
                let lens = Lens { zoom: viewing.zoom.scale, pan: (viewing.zoom.pan.x, viewing.zoom.pan.y) };
                Some(div().absolute().inset_0().child(crate::scene::view_marked(SharedString::from(held), scene, None, Fit::Reading, Clock::default(), lens, self.note_marks())).into_any_element())
            }
            (_, Some((held, scene))) => {
                let lens = Lens { zoom: viewing.zoom.scale, pan: (viewing.zoom.pan.x, viewing.zoom.pan.y) };
                Some(div().absolute().inset_0().child(crate::scene::view(SharedString::from(held), scene, Fit::Reading, Clock::default(), lens)).into_any_element())
            }
            _ => None,
        };
        let name = self.picture_name(&viewing.what);
        let drawing = matches!(viewing.what, What::Drawing { .. });
        // On a drawing, a press that did not travel picks the part under it, and a right press offers a note on it.
        let body = zoomer::bind(div().id("picture-body").relative().flex_1().min_h_0().overflow_hidden().bg(look.color(token::BG_BASE)), cx, |this, at, _, cx| {
            if this.viewed_drawing().is_some() {
                let part = this.note_part_at(at);
                this.pick_part(part, cx);
            }
        })
        .when(drawing, |body| {
            body.on_mouse_down(MouseButton::Right, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                let part = this.note_part_at(event.position);
                this.note_menu(event.position, part, window, cx);
                cx.stop_propagation();
            }))
        })
            .child(viewing.zoom.probe())
            .children(content)
            .children(readout.map(|readout| zoomer::corner(look, readout, |this, _, cx| zoomer::refit(this, cx), cx)));
        let note_box = if drawing { self.render_note_box(area, look, cx) } else { None };
        let view = labelled(
            div()
                .id("picture-view")
                .relative()
                .occlude()
                .track_focus(&viewing.focus)
                .flex()
                .flex_col()
                .w(area.size.width)
                .h(area.size.height)
                .bg(look.color(token::BG_BASE))
                .on_key_down(cx.listener(Self::picture_key))
                .child(head)
                .child(body)
                .children(foot)
                .children(note_box),
            Role::Dialog,
            name,
        );
        // Opened from a card, the view grows out of where the card stood, as if it zoomed out of the thread.
        let view = match viewing.from {
            Some((from, opens)) => grow(view, from, area, look.length(token::RADIUS_LG), look.duration(token::DURATION_ENTER), opens),
            None => view.into_any_element(),
        };
        Some(deferred(anchored().position(area.origin).anchor(Anchor::TopLeft).child(view)).with_priority(3))
    }

    /// The stage the open picture stands on in a view of `view` size, where it is drawn.
    fn stage_of(&self, viewing: &Viewing, view: Size<f32>, cx: &App) -> Option<Stage> {
        match &viewing.what {
            What::Raster { natural, .. } => Some(Stage::of_picture(*natural, view, f32::from(cx.look().length(token::SPACE_6)))),
            what => {
                let (held, _) = self.scene_of(what)?;
                let (fit, actual) = crate::scene::fit_of(&held)?;
                Some(Stage { view, fit, actual })
            }
        }
    }
}

/// `view`, standing over `area`, grows out of `from`: it starts at the card's place and size, rounded as the card is, and eases out
/// to fill the area. Each open (`opens`) grows afresh.
fn grow(view: impl IntoElement + 'static, from: Bounds<Pixels>, area: Bounds<Pixels>, radius: Pixels, over: std::time::Duration, opens: usize) -> AnyElement {
    let lerp = |a: Pixels, b: Pixels, at: f32| a + (b - a) * at;
    div()
        .overflow_hidden()
        .child(view)
        .with_animation(SharedString::from(format!("picture-grow:{opens}")), Animation::new(over).with_easing(gpui::ease_out_quint()), move |frame, at| {
            frame
                .relative()
                .left(lerp(from.left() - area.left(), px(0.), at))
                .top(lerp(from.top() - area.top(), px(0.), at))
                .w(lerp(from.size.width, area.size.width, at))
                .h(lerp(from.size.height, area.size.height, at))
                .rounded(lerp(radius, px(0.), at))
                .opacity(0.4 + 0.6 * at)
        })
        .into_any_element()
}

impl Zooms for JobView {
    fn zoomer(&mut self) -> Option<&mut Zoomer> {
        if let Some(viewing) = self.pictures.viewing.as_mut() {
            return Some(&mut viewing.zoom);
        }
        if let Some(reading) = self.sketch_reading.as_mut() {
            return Some(&mut reading.zoom);
        }
        self.detail_zoom.as_mut().map(|detail| &mut detail.zoom)
    }

    fn stage(&self, view: Size<f32>, _: &Window, cx: &App) -> Option<Stage> {
        if let Some(viewing) = self.pictures.viewing.as_ref() {
            return self.stage_of(viewing, view, cx);
        }
        if let Some(reading) = self.sketch_reading.as_ref() {
            let (fit, actual) = crate::scene::fit_of(&Self::drawn_key(reading))?;
            return Some(Stage { view, fit, actual });
        }
        let detail = self.detail_zoom.as_ref()?;
        let (fit, actual) = crate::scene::fit_of(&detail.held.borrow())?;
        Some(Stage { view, fit, actual })
    }
}

#[cfg(test)]
mod tests {
    use super::super::parts_card::part_key;
    use super::super::parts_fold::{drawn, redraw, rooted_job};
    use super::*;
    use crate::mock_parts::{ID, JOB};

    const A: &str = "A \u{b7} Bar under the title";
    const B: &str = "B \u{b7} Rail";

    fn takes_view<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        // Over the review sheet, the one place the vote is answered.
        view.update_in(cx, |view, window, cx| view.open_artifact_review(ID, window, cx));
        redraw(cx);
        view.update_in(cx, |view, window, cx| view.open_take(&part_key(ID, "bar"), 1, window, cx));
        redraw(cx);
        (view, cx)
    }

    /// His note of 23 Nov: an artifact that asks is read and answered in the sheet alone, so a tab kept for it is let go.
    #[gpui::test]
    fn an_artifact_that_asks_keeps_no_tab_of_its_own(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_artifact_reader(ID, window, cx));
        redraw(cx);
        let tabbed = view.read_with(cx, |view, _| view.row_tab_list(JOB).iter().any(|tab| tab.key == ID));
        assert!(!tabbed, "the tab is let go while the artifact asks");
        assert!(!drawn(cx, Role::Region, "Reader: Step bar for Jobs, Revision 2"), "and nothing reads it outside the sheet");
    }

    /// The last drawn with this role and name: with the review panel up, the thread behind draws its own beneath the panel's.
    fn bounds_of(cx: &mut gpui::VisualTestContext, role: Role, name: &str) -> Bounds<Pixels> {
        cx.update(|window, cx| cx.global::<crate::labels::LabelBook>().drawn(window.window_handle().window_id()).iter().rev().find(|one| one.role == role && one.name == name).map(|one| one.bounds))
            .unwrap_or_else(|| panic!("{name} was not drawn"))
    }

    #[test]
    fn a_take_steps_to_its_neighbours_and_round_the_ends() {
        assert_eq!(stepped(0, -1, 3), 2);
        assert_eq!(stepped(2, 1, 3), 0);
        assert_eq!(stepped(1, 1, 3), 2);
        assert_eq!(stepped(0, 1, 0), 0);
    }

    #[test]
    fn a_take_is_named_by_its_place_in_quiet_words() {
        assert_eq!(take_of(1, 3), "Take B of 3");
        assert_eq!(take_of(0, 2), "Take A of 2");
    }

    #[gpui::test]
    fn a_take_opens_below_the_title_bar_named_with_both_neighbours_and_a_pick_button(cx: &mut gpui::TestAppContext) {
        let (_, cx) = takes_view(cx);
        let (area, bar) = cx.update(|window, cx| (app_area(window, cx), cx.look().length(token::H_TITLE_BAR)));
        assert_eq!(area.origin.y, bar, "it starts under the title bar");
        assert_eq!(area.size.height, px(900.) - bar, "and runs to the window's foot");
        assert_eq!(bounds_of(cx, Role::Dialog, B).size, area.size, "the view is the whole of that area");
        assert!(drawn(cx, Role::Heading, B), "the head names the take by its own name");
        assert!(drawn(cx, Role::Button, A), "the take before it is named in the head");
        assert!(drawn(cx, Role::Button, "C \u{b7} Dots, set out in a row of three, a title long enough to run past a whole line of the strip and be cut after the second"), "and the take after");
        assert!(drawn(cx, Role::Button, "Picked"), "the pick stands in the view, on the Job's pick, where the sheet's vote starts");
        assert!(drawn(cx, Role::Button, "Close"));
    }

    #[gpui::test]
    fn the_arrow_keys_step_between_takes_and_escape_closes_the_view_and_gives_the_keys_back(cx: &mut gpui::TestAppContext) {
        let (view, cx) = takes_view(cx);
        let before = view.read_with(cx, |view, _| view.pictures.viewing.as_ref().and_then(|viewing| viewing.back.clone()));
        cx.simulate_keystrokes("left");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, A), "left steps to the take before");
        cx.simulate_keystrokes("left");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, "C \u{b7} Dots, set out in a row of three, a title long enough to run past a whole line of the strip and be cut after the second"), "round the end");
        cx.simulate_keystrokes("right right");
        redraw(cx);
        assert!(drawn(cx, Role::Heading, A) || drawn(cx, Role::Heading, B), "right steps on");
        cx.simulate_keystrokes("escape");
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.pictures.viewing.is_none()), "Esc closes it");
        assert!(!drawn(cx, Role::Dialog, A) && !drawn(cx, Role::Dialog, B));
        let focused = view.update_in(cx, |_, window, cx| window.focused(cx));
        assert!(focused == before, "the keys are back where they were");
    }

    #[gpui::test]
    fn pick_marks_the_take_and_the_view_keeps_send_pick_at_its_foot(cx: &mut gpui::TestAppContext) {
        let (view, cx) = takes_view(cx);
        assert!(drawn(cx, Role::Button, "Picked"), "over the sheet the vote starts on the Job's pick, the take shown");
        cx.simulate_keystrokes("right");
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Pick"), "another take is not marked");
        view.update_in(cx, |view, _, cx| view.choose_take(&part_key(ID, "bar"), 2, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Picked"), "the pick is marked on the take he picked");
        assert!(drawn(cx, Role::Button, "Send pick"), "and the vote's own act is reachable from the view");
    }

    #[gpui::test]
    fn a_take_in_the_reader_fills_the_column_and_the_takes_stand_one_to_a_row(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(900.)), |_| {});
        // Read whole in its own tab once it asks nothing: while it asks, the sheet alone holds it.
        view.update(cx, |view, cx| {
            for part in view.artifact_mut(JOB, ID).map(|one| one.parts.iter_mut()).into_iter().flatten() {
                part.asks = false;
            }
            view.parts_changed(ID, cx);
        });
        view.update_in(cx, |view, window, cx| view.open_artifact_reader(ID, window, cx));
        redraw(cx);
        let (first, second) = (bounds_of(cx, Role::Button, &format!("Picture: {A}")), bounds_of(cx, Role::Button, &format!("Picture: {B}")));
        assert!(second.top() >= first.bottom(), "one take to a row: {first:?} then {second:?}");
        let tile = bounds_of(cx, Role::RadioButton, &format!("Take {A}"));
        assert!(first.size.width >= tile.size.width - px(60.), "the picture takes the tile's width: {:?} in {:?}", first.size, tile.size);
        view.update_in(cx, |view, window, cx| view.open_take(&part_key(ID, "bar"), 0, window, cx));
        redraw(cx);
        assert!(drawn(cx, Role::Dialog, A), "a press on the picture opens it over the reader");
    }

    /// The Job's thread with the step bar's card drawn whole, its parts only informing.
    fn informing_card<'a>(cx: &'a mut gpui::TestAppContext) -> (gpui::Entity<JobView>, &'a mut gpui::VisualTestContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(6000.)), |_| {});
        view.update(cx, |view, cx| {
            for part in view.artifact_mut(JOB, ID).map(|one| one.parts.iter_mut()).into_iter().flatten() {
                part.asks = false;
            }
            view.parts_changed(ID, cx);
        });
        redraw(cx);
        (view, cx)
    }

    #[gpui::test]
    fn a_drawing_in_a_thread_card_fills_the_card_with_no_frame_of_its_own_and_a_press_opens_it_large(cx: &mut gpui::TestAppContext) {
        let (view, cx) = informing_card(cx);
        let card = bounds_of(cx, Role::Article, "Artifact: Step bar for Jobs, Revision 2");
        let drawing = bounds_of(cx, Role::Button, "Drawing");
        let pad = cx.update(|_, cx| cx.look().length(token::SPACE_4));
        assert!(drawing.size.width >= card.size.width - pad * 2. - px(4.), "the drawing takes the card's inner width: {drawing:?} in {card:?}");
        let old_height = cx.update(|_, cx| cx.look().length(token::SPACE_12) * 3.);
        assert!(drawing.size.height != old_height, "its height is the scene's own shape, not a fixed box: {drawing:?}");
        let _ = view;
        cx.simulate_click(drawing.center(), gpui::Modifiers::none());
        redraw(cx);
        assert!(drawn(cx, Role::Button, "Fit"), "a press on it opens the app-filling view");
    }

    const CARD: &str = "Artifact: Step bar for Jobs, Revision 2";

    #[gpui::test]
    fn a_press_anywhere_on_the_card_grows_its_drawing_out_of_the_card_into_the_app_filling_view(cx: &mut gpui::TestAppContext) {
        let (view, cx) = informing_card(cx);
        let card = bounds_of(cx, Role::Article, CARD);
        cx.simulate_click(card.origin + gpui::point(px(6.), px(6.)), gpui::Modifiers::none());
        redraw(cx);
        let from = view.read_with(cx, |view, _| view.pictures.viewing.as_ref().and_then(|viewing| viewing.from.map(|(from, _)| from)));
        let from = from.expect("a press on the card's edge opens the view, grown from the card");
        assert!((from.top() - card.top()).abs() <= px(2.) && (from.size.width - card.size.width).abs() <= px(2.), "it starts where the card stood, inside its border: {from:?} for {card:?}");
        assert!(view.read_with(cx, |view, _| view.viewed_drawing().is_some()), "it shows the card's drawing");
        assert!(drawn(cx, Role::Button, "Fit"), "and pans and zooms as the large view does");
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        redraw(cx);
        let area = cx.update(|window, cx| app_area(window, cx));
        let name = view.read_with(cx, |view, _| view.pictures.viewing.as_ref().map(|viewing| view.picture_name(&viewing.what))).expect("still open");
        assert_eq!(bounds_of(cx, Role::Dialog, &name).size, area.size, "grown, it fills the app");
    }

    #[gpui::test]
    fn a_press_on_an_act_of_the_card_does_its_act_and_grows_nothing(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1400.), px(6000.)), |_| {});
        // The card's acts, in the review sheet.
        view.update_in(cx, |view, window, cx| view.open_artifact_review(ID, window, cx));
        redraw(cx);
        let act = bounds_of(cx, Role::Button, "Ask for changes");
        cx.simulate_click(act.center(), gpui::Modifiers::none());
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.pictures.viewing.is_none()), "the act's press stays with the act");
    }

    #[gpui::test]
    fn a_right_press_on_a_part_of_the_grown_drawing_picks_it_for_a_note_on_the_artifact(cx: &mut gpui::TestAppContext) {
        let (view, cx) = informing_card(cx);
        let card = bounds_of(cx, Role::Article, CARD);
        cx.simulate_click(card.origin + gpui::point(px(6.), px(6.)), gpui::Modifiers::none());
        cx.executor().advance_clock(std::time::Duration::from_secs(1));
        redraw(cx);
        let drawn_key = view.read_with(cx, |view, _| view.viewed_drawing().map(|(_, drawn, _)| drawn)).expect("a drawing is shown");
        let (part, at) = crate::scene::nodes(&drawn_key).into_iter().next().expect("the drawing has a node");
        cx.simulate_event(MouseDownEvent { button: MouseButton::Right, position: at.center(), modifiers: gpui::Modifiers::none(), click_count: 1, first_mouse: false });
        redraw(cx);
        let target = view.read_with(cx, |view, _| view.pictures.viewing.as_ref().and_then(|viewing| viewing.target.clone()));
        assert_eq!(target.map(|one| one.id), Some(part.id.clone()), "the part under the right press is picked");
        let marks = view.read_with(cx, |view, _| view.note_marks());
        assert!(marks.picked.contains(&part.id), "and ringed on the drawing");
    }

    #[gpui::test]
    fn a_tall_take_in_the_review_panel_stands_as_tall_as_a_wide_one_and_no_taller_than_its_share(cx: &mut gpui::TestAppContext) {
        let (view, cx) = rooted_job(cx, "artifact-parts", JOB, size(px(1100.), px(700.)), |_| {});
        view.update_in(cx, |view, window, cx| view.open_artifact_tab(ID, window, cx));
        redraw(cx);
        assert!(view.read_with(cx, |view, _| view.review_open()), "the take is read in the review panel");
        let panel = view.read_with(cx, |view, _| view.sheet.get().height);
        assert!(panel > px(0.), "the panel has a height");
        // The Job's pick starts chosen; a take's tile is its picture and its name, one press for the pick.
        let tall = bounds_of(cx, Role::RadioButton, &format!("Chosen: {B}"));
        let wide = bounds_of(cx, Role::RadioButton, &format!("Pick {A}"));
        assert!((tall.size.height - wide.size.height).abs() < px(1.), "every well is one height, so the takes stand level: {tall:?} beside {wide:?}");
        assert!(tall.size.height <= panel * super::super::reader_sheet::TALL_SHARE + px(1.), "no taller than its share of the panel: {tall:?} in a panel {panel:?} high");
    }

    #[gpui::test]
    fn takes_in_a_thread_card_stand_one_a_row_fill_it_and_a_press_opens_that_take_large(cx: &mut gpui::TestAppContext) {
        let (_, cx) = informing_card(cx);
        let (first, second) = (bounds_of(cx, Role::Button, &format!("Picture: {A}")), bounds_of(cx, Role::Button, &format!("Picture: {B}")));
        assert!(second.top() >= first.bottom(), "a thread too narrow for 520 a take stands them one to a row: {first:?} then {second:?}");
        let tile = bounds_of(cx, Role::RadioButton, &format!("Take {A}"));
        assert!(first.size.width >= tile.size.width - px(60.), "the picture takes the tile's width: {:?} in {:?}", first.size, tile.size);
        let old_height = cx.update(|_, cx| cx.look().length(token::SPACE_12) * 2.);
        assert!(first.size.height != old_height, "its height is the scene's own shape, not the old fixed box: {first:?}");
        cx.simulate_click(first.center(), gpui::Modifiers::none());
        redraw(cx);
        assert!(drawn(cx, Role::Dialog, A), "a press on the picture opens that take over the app");
    }
}
