//! A question whose options are drawn as pictures, taken in from the Needs you band (take A, owner, 13 Nov). The
//! band holds one row for it: the stills small, the question, and Look and choose (↩). That opens the options over
//! the thread, each still large with its words and Choose under it. Choose and `1` to `9` answer; ← and → light the
//! next option; a press on a still opens it full size (`option_pictures::large`) and answers nothing.
//!
//! The answer goes as the band's own option press sends it (`answer_row`). Answer in words sets the box to answer
//! the question, and what he sends goes as the panel's Answer in words box sends it (`answer_row_words`).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{canvas, div, prelude::*, AnyElement, Context, Div, FontWeight, KeyDownEvent, Pixels, Role, SharedString, Size, Stateful, Window};
use ipc::{NeedsYouAct, NeedsYouRow, WaitingOption};

use super::dock::{row_options, Thing};
use super::JobView;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::operator::view::{press, Weight};
use crate::titlebar::parts::kbd;

/// The options open over the thread: the question's row, the option ← and → have lit, and what was last laid out,
/// so each still takes its column's width and the height its words leave.
pub(super) struct Choosing {
    pub item: String,
    pub lit: usize,
    /// The room the options have inside the sheet.
    area: Rc<Cell<Size<Pixels>>>,
    /// How tall each pictured option's words were drawn.
    words: Rc<RefCell<HashMap<usize, f32>>>,
    /// How tall the options in words under the stills were drawn, together.
    worded: Rc<Cell<f32>>,
}

/// Which of `count` stills stand side by side when `fits` do: all when they fit, else a run that holds the lit one.
pub(super) fn shown_run(lit: Option<usize>, count: usize, fits: usize) -> std::ops::Range<usize> {
    let fits = fits.clamp(1, count.max(1));
    let start = match lit {
        Some(at) if at >= fits => (at + 1 - fits).min(count.saturating_sub(fits)),
        _ => 0,
    };
    start..(start + fits).min(count)
}

/// How many things `least` wide, `gap` apart, stand in `across`; never fewer than one.
pub(super) fn how_many_fit(across: f32, least: f32, gap: f32) -> usize {
    (((across + gap) / (least + gap)).floor() as usize).max(1)
}

/// The band's second line: how many options are drawn, in words.
pub(super) fn drawn_in_words(count: usize) -> String {
    const WORDS: [&str; 9] = ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine"];
    let number = count.checked_sub(1).and_then(|at| WORDS.get(at)).map_or_else(|| count.to_string(), |word| (*word).to_string());
    if count == 1 { format!("{number} option drawn as a picture") } else { format!("{number} options drawn as pictures") }
}

/// The option lit after ← (`by` -1) or → (`by` 1), wrapping at either end as the full-size view does.
pub(super) fn step_lit(lit: usize, by: isize, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    (lit as isize + by).rem_euclid(count as isize) as usize
}

/// The option a number key chooses: `1` is the first, and a number past the last chooses nothing.
pub(super) fn chosen_by_key(key: &str, count: usize) -> Option<usize> {
    let n: usize = key.parse().ok()?;
    (1..=count.min(9)).contains(&n).then(|| n - 1)
}

/// The size a still is drawn at: its column's whole inner width `across`, unless the height `down` left under its
/// words is less. Its shape is kept.
pub(super) fn fit(width: u32, height: u32, across: f32, down: f32) -> Option<(f32, f32)> {
    let (w, h) = (width.max(1) as f32, height.max(1) as f32);
    if across < 1. || down < 1. {
        return None;
    }
    let scale = (across / w).min(down / h);
    Some((w * scale, h * scale))
}

/// What a press does on a click, and on Enter or Space while it has the keys.
fn pressed(element: Stateful<Div>, cx: &Context<JobView>, act: impl Fn(&mut JobView, &mut Window, &mut Context<JobView>) + 'static) -> Stateful<Div> {
    let act = Rc::new(act);
    let by_key = act.clone();
    element.on_click(cx.listener(move |this, _, window, cx| act(this, window, cx))).on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
        let key = &event.keystroke;
        if matches!(key.key.as_str(), "enter" | "space") && !key.modifiers.modified() {
            cx.stop_propagation();
            by_key(this, window, cx);
        }
    }))
}

/// A press filled in the waiting amber: the one the band or the sheet leads with.
fn primary(element: Stateful<Div>, look: &Look) -> Stateful<Div> {
    let (hue, ink) = (look.color(token::STEP_WAITING), look.color(token::FG_INVERSE));
    element
        .bg(hue)
        .border_color(hue)
        .text_color(ink)
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .hover(move |style| style.bg(hue.opacity(0.88)))
}

impl JobView {
    /// The front question, when the asker drew any of its options as a still: the first question Needs you lists.
    pub(super) fn pictured_front<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Option<&'a NeedsYouRow> {
        let (things, _) = self.things(id, cx);
        let front = things.into_iter().find(|thing| match thing {
            Thing::Ask(_) | Thing::Lapsed(_) => true,
            Thing::Row(row) => matches!(row.act, NeedsYouAct::Answer { .. }) && !row_options(row).is_empty(),
        })?;
        match front {
            Thing::Row(row) if row_options(row).iter().any(|option| option.picture.is_some()) => Some(row),
            _ => None,
        }
    }

    /// The band's row for a question asked in pictures: its stills small, the question, and Look and choose.
    pub(super) fn render_look_row(&self, id: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        // While its options are open over the thread the row would say the same again; the sheet takes its room.
        // So does it while the box answers the question, which the line over the box names.
        if self.dock_of(id).is_some_and(|dock| dock.choosing.is_some()) || self.answering_row(id, cx).is_some() {
            return None;
        }
        let row = self.pictured_front(id, cx)?;
        let options = row_options(row);
        let port = self.port(cx);
        let hue = look.color(token::STEP_WAITING);
        let size = (f32::from(look.length(token::SPACE_12) + look.length(token::SPACE_4)), f32::from(look.length(token::SPACE_8) + look.length(token::SPACE_2)));
        let drawn = crate::option_pictures::drawn(options);
        let count = drawn.len();
        // The question and Look and choose keep their room; the thumbnails take what is left, and none in a narrow band.
        let across = self.dock_of(id).map(|dock| dock.look_row.clone()).unwrap_or_default();
        let kept = f32::from(look.length(token::SPACE_12)) * 8.;
        let gap = f32::from(look.length(token::SPACE_1));
        let room = across.get() - kept;
        let thumbs_fit = if room < size.0 { 0 } else { how_many_fit(room, size.0, gap) };
        let thumbs = drawn.into_iter().take(thumbs_fit).filter_map(|(at, _)| crate::option_pictures::thumb(&row.id, options, at, size, port, look));
        let open = labelled(
            pressed(primary(press("look-and-choose", "Look and choose".into(), None, None, Weight::Plain, look), look), cx, |this, window, cx| this.open_choose(window, cx))
                .child(kbd("\u{21a9}", look).bg(look.color(token::FG_INVERSE).opacity(0.12)).border_color(look.color(token::FG_INVERSE).opacity(0.3)).text_color(look.color(token::FG_INVERSE))),
            Role::Button,
            "Look and choose",
        );
        let measured = across.clone();
        Some(
            labelled(
                div()
                    .id("needs-look")
                    .relative()
                    .flex_none()
                    .flex()
                    // A narrow band puts Look and choose on a line of its own, so the question keeps its words.
                    .flex_wrap()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .bg(hue.opacity(0.1))
                    .border_b(look.length(token::BORDER_WIDTH))
                    .border_color(hue.opacity(0.4))
                    .when(thumbs_fit > 0, |band| band.child(div().flex_none().flex().gap(look.length(token::SPACE_1)).children(thumbs)))
                    .child(
                        div()
                            .flex_1()
                            .min_w(look.length(token::SPACE_12) * 4.)
                            .flex()
                            .flex_col()
                            .child(div().line_clamp(2).text_ellipsis().text_size(look.length(token::TEXT_SM)).line_height(look.length(token::LEADING_SM)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.text.clone())))
                            .child(div().line_clamp(2).text_ellipsis().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(drawn_in_words(count))),
                    )
                    .child(div().flex_none().ml_auto().child(open))
                    .child(measure(move |size, window| {
                        if measured.replace(f32::from(size.width)) != f32::from(size.width) {
                            window.refresh();
                        }
                    })),
                Role::Group,
                row.text.clone(),
            )
            .into_any_element(),
        )
    }

    /// Look and choose, or Return in an empty box: the options open over the thread with the Job's pick lit, and the
    /// Needs you panel folds so the two never lie one over the other.
    pub(super) fn open_choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some((item, lit)) = self.pictured_front(&id, cx).map(|row| (row.id.clone(), crate::ways::pick_of(row_options(row)).unwrap_or(0))) else { return };
        if let Some(dock) = self.dock_mut() {
            dock.open = Some(false);
            dock.detail = None;
            dock.choosing = Some(Choosing { item, lit, area: Rc::default(), words: Rc::default(), worded: Rc::default() });
            dock.answering = None;
        }
        window.focus(&self.choose_focus, cx);
        cx.notify();
    }

    /// Esc, or Answer in words: the options go and the caret is in the box.
    pub(super) fn close_choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(dock) = self.dock_mut() {
            dock.choosing = None;
        }
        self.to_composer(window, cx);
        cx.notify();
    }

    /// Answer in words: the options go, and the box answers the question until he sends or presses Esc.
    fn words_for_choosing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.dock_mut().and_then(|dock| dock.choosing.take()).map(|open| open.item) else { return };
        if let Some(dock) = self.dock_mut() {
            dock.answering = Some(item);
        }
        self.to_composer(window, cx);
        cx.notify();
    }

    /// The question the box answers now, while it is still waiting.
    pub(super) fn answering_row<'a>(&'a self, id: &str, cx: &'a gpui::App) -> Option<&'a NeedsYouRow> {
        let item = self.dock_of(id)?.answering.as_ref()?;
        self.choosing_row(id, item, cx)
    }

    /// Esc in the box while it answers: it is a plain message again, its words kept.
    pub(super) fn drop_answering(&mut self, cx: &mut Context<Self>) -> bool {
        let held = self.dock_mut().and_then(|dock| dock.answering.take()).is_some();
        if held {
            cx.notify();
        }
        held
    }

    /// Over the box while it answers: what it answers, and the way back to a message.
    pub(super) fn render_answering(&self, id: &str, look: &Look, cx: &Context<Self>) -> Option<AnyElement> {
        let row = self.answering_row(id, cx)?;
        let hue = look.color(token::STEP_WAITING);
        let back = labelled(
            pressed(
                div().id("answering-back").flex_none().cursor_pointer().rounded(look.length(token::RADIUS_SM)).child(kbd("Esc", look)),
                cx,
                |this, _, cx| {
                    this.drop_answering(cx);
                },
            )
            .focusable()
            .tooltip(crate::titlebar::parts::tip("Message instead")),
            Role::Button,
            "Message instead",
        );
        Some(
            labelled(
                div()
                    .id("answering")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .px(look.length(token::SPACE_4))
                    .pt(look.length(token::SPACE_2))
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(hue).child("Answer in words"))
                            .child(div().line_clamp(2).text_ellipsis().text_color(look.color(token::FG_MUTED)).child(SharedString::from(row.text.clone()))),
                    )
                    .child(back),
                Role::Status,
                format!("Answer in words: {}", row.text),
            )
            .into_any_element(),
        )
    }

    /// Choose, or its number: the option goes as the band's press sends it, and the band moves on.
    fn choose_option(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.dock_mut().and_then(|dock| dock.choosing.take()).map(|open| open.item) else { return };
        self.answer_row(&item, at, window, cx);
        self.to_composer(window, cx);
    }

    fn on_choose_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.shown.clone() else { return };
        let Some(count) = self.dock_of(&id).and_then(|dock| dock.choosing.as_ref()).and_then(|open| self.choosing_row(&id, &open.item, cx)).map(|row| row_options(row).len()) else { return };
        let key = &event.keystroke;
        if key.modifiers.modified() {
            return;
        }
        match key.key.as_str() {
            "escape" => self.close_choose(window, cx),
            "left" | "right" => {
                let by = if key.key == "left" { -1 } else { 1 };
                if let Some(open) = self.dock_mut().and_then(|dock| dock.choosing.as_mut()) {
                    open.lit = step_lit(open.lit, by, count);
                }
                cx.notify();
            }
            digit => match chosen_by_key(digit, count) {
                Some(at) => self.choose_option(at, window, cx),
                None => return,
            },
        }
        cx.stop_propagation();
    }

    fn choosing_row<'a>(&'a self, id: &str, item: &str, cx: &'a gpui::App) -> Option<&'a NeedsYouRow> {
        let (things, _) = self.things(id, cx);
        things.into_iter().find_map(|thing| match thing {
            Thing::Row(row) if row.id == item => Some(row),
            _ => None,
        })
    }

    /// The options over the thread, from its top to the band: the question and the Job, each option side by side as
    /// its still large with its words and Choose, options in words under them, and the keys at the foot.
    pub(super) fn render_choose(&self, id: &str, cx: &Context<Self>) -> Option<AnyElement> {
        let open = self.dock_of(id)?.choosing.as_ref()?;
        let row = self.choosing_row(id, &open.item, cx)?;
        let look = cx.look();
        let options = row_options(row);
        let hue = look.color(token::STEP_WAITING);
        let port = self.port(cx);
        let pick = crate::ways::pick_of(options).unwrap_or(0);
        let title = self.title_of(id, cx).unwrap_or_default();
        let question = SharedString::from(row.text.clone());
        let area = open.area.get();
        let (across, down) = (f32::from(area.width), f32::from(area.height));
        // Narrower than this the Job's name goes under the question, and the foot keeps only ← → and Answer in words.
        let narrow = across < f32::from(look.length(token::SPACE_12)) * 9.;
        let choose_of = |at: usize, option: &WaitingOption| {
            let label = crate::ways::name_of(&option.label).to_string();
            let mut choose = press(format!("choose:{}:{at}", row.id), "Choose".into(), None, None, Weight::Plain, look);
            if at == pick {
                choose = primary(choose, look);
            }
            labelled(pressed(choose, cx, move |this, window, cx| this.choose_option(at, window, cx)), Role::Button, format!("Choose {label}"))
        };
        let number = |at: usize| div().flex_none().text_size(look.length(token::TEXT_2XS)).text_color(look.color(token::FG_SUBTLE)).child(format!("{}", at + 1));
        let name = |option: &WaitingOption| {
            div().line_clamp(2).text_ellipsis().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(crate::ways::name_of(&option.label).to_string()))
        };
        let said_of = |option: &WaitingOption, lines: usize| {
            option.description.as_ref().filter(|said| !said.is_empty()).map(|said| {
                div().line_clamp(lines).text_ellipsis().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone()))
            })
        };
        // An option in words under the stills: its number, its words, and Choose at the right.
        let words_in_row = |at: usize, option: &WaitingOption| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .child(number(at))
                .child(div().flex_1().min_w_0().flex().flex_col().child(name(option)).children(said_of(option, 2)))
                .child(choose_of(at, option))
        };
        // An option in a column: its number and label on one line, its words under them, measured so the still knows
        // the height they take.
        let words_in_column = |at: usize, option: &WaitingOption| {
            let words = open.words.clone();
            div()
                .relative()
                .flex()
                .flex_col()
                .child(div().flex().items_baseline().gap(look.length(token::SPACE_2)).child(number(at)).child(name(option).flex_1().min_w_0()))
                .children(said_of(option, 3))
                .child(measure(move |size, window| {
                    let high = f32::from(size.height);
                    if words.borrow_mut().insert(at, high) != Some(high) {
                        window.refresh();
                    }
                }))
        };
        let pad = look.length(token::SPACE_2);
        let edge = look.length(token::BORDER_WIDTH);
        let frame = |at: usize, option: &WaitingOption, body: Div, grow: bool| {
            let lit = open.lit == at;
            let label = crate::ways::name_of(&option.label).to_string();
            labelled(
                div()
                    .id(SharedString::from(format!("choose-option:{at}")))
                    .map(|one| if grow { one.flex_1().min_w_0() } else { one.flex_none() })
                    .flex()
                    .flex_col()
                    .gap(pad)
                    .p(pad)
                    .rounded(look.length(token::RADIUS_LG) * 1.5)
                    .border(edge)
                    .border_color(if lit { hue.opacity(0.55) } else { look.color(token::BORDER_SUBTLE) })
                    .bg(if lit { hue.opacity(0.12) } else { look.color(token::FG_DEFAULT).opacity(0.02) })
                    .child(body),
                Role::Group,
                if lit { format!("{label}, selected") } else { label },
            )
        };
        let (pictured, worded): (Vec<_>, Vec<_>) = options.iter().enumerate().partition(|(_, option)| option.picture.is_some());
        let gap = look.length(token::SPACE_3);
        let fits = how_many_fit(across, f32::from(look.length(token::SPACE_12)) * 3.5, f32::from(gap));
        // When the stills do not all fit side by side, every option takes its turn in the same place, one run at a time,
        // options in words among them; otherwise the options in words stand under the stills.
        let turns = fits < pictured.len();
        let (pool, worded) = if turns { (options.iter().enumerate().collect::<Vec<_>>(), Vec::new()) } else { (pictured, worded) };
        let run = if turns { shown_run(Some(open.lit), pool.len(), fits) } else { 0..pool.len() };
        let shown = run.len().max(1) as f32;
        // Each still takes its column's inner width, unless the height its words and the options in words leave is less.
        let inner = (across - f32::from(gap) * (shown - 1.)) / shown - f32::from(pad + edge) * 2.;
        let under = if worded.is_empty() { 0. } else { open.worded.get() + f32::from(look.length(token::SPACE_2)) };
        let least = f32::from(look.length(token::SPACE_12)) * 1.5;
        let columns = pool[run].iter().map(|&(at, option)| {
            let words_high = open.words.borrow().get(&at).copied().unwrap_or(0.);
            let left = (down - under - f32::from(pad + edge) * 2. - f32::from(pad) * 2. - words_high - f32::from(look.length(token::H_CONTROL_LG))).max(least);
            let still = option
                .picture
                .as_ref()
                .and_then(|picture| fit(picture.width, picture.height, inner, left))
                .and_then(|size| crate::option_pictures::large(&row.id, options, at, size, open.lit == at, port, hue, look))
                .map(|still| div().flex().justify_center().child(still));
            // Choose takes the foot of its column, so the columns, as tall as the tallest, hold it on one line.
            let body = div().flex_1().flex().flex_col().gap(pad).children(still).child(words_in_column(at, option)).child(div().mt_auto().flex().child(choose_of(at, option)));
            frame(at, option, body, true).into_any_element()
        });
        let (measured_area, measured_worded) = (open.area.clone(), open.worded.clone());
        let close = labelled(
            pressed(div().id("choose-close").flex_none().cursor_pointer().rounded(look.length(token::RADIUS_SM)).child(kbd("Esc", look)), cx, |this, window, cx| this.close_choose(window, cx))
                .focusable()
                .tooltip(crate::titlebar::parts::tip("Close")),
            Role::Button,
            "Close",
        );
        let said = div().flex_1().min_w_0().line_clamp(2).text_ellipsis().text_size(look.length(token::TEXT_BASE)).line_height(look.length(token::LEADING_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(question.clone());
        let from = div().min_w_0().truncate().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(token::FG_MUTED)).child(title);
        let top = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_2))
            .bg(hue.opacity(0.12))
            .border_b(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .map(|top| if narrow { top.child(div().flex_1().min_w_0().flex().flex_col().child(said).child(from)) } else { top.child(said).child(from.flex_none().max_w(look.length(token::SPACE_12) * 5.)) })
            .child(close);
        let body = div()
            .relative()
            .flex_1()
            .min_h_0()
            .m(look.length(token::SPACE_2))
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .overflow_hidden()
            .child(div().flex().gap(gap).children(columns))
            .when(!worded.is_empty(), |body| {
                body.child(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .gap(look.length(token::SPACE_2))
                        .children(worded.into_iter().map(|(at, option)| frame(at, option, words_in_row(at, option), false).into_any_element()))
                        .child(measure(move |size, window| {
                            let high = f32::from(size.height);
                            if measured_worded.replace(high) != high {
                                window.refresh();
                            }
                        })),
                )
            })
            .child(measure(move |size, window| {
                if measured_area.replace(size) != size {
                    window.refresh();
                }
            }));
        let keys = (1..=options.len().min(9)).map(|n| kbd(n.to_string(), look));
        let words = labelled(
            pressed(press("choose-words", "Answer in words".into(), None, None, Weight::Plain, look), cx, |this, window, cx| this.words_for_choosing(window, cx)),
            Role::Button,
            "Answer in words",
        );
        let foot = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_1))
            .border_t(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_SUBTLE))
            .text_size(look.length(token::TEXT_2XS))
            .text_color(look.color(token::FG_MUTED))
            .child(kbd("\u{2190}", look))
            .child(kbd("\u{2192}", look))
            .when(!narrow, |foot| foot.child(div().mr(look.length(token::SPACE_2)).child("between options")).children(keys).child("choose"))
            .child(div().flex_1())
            .child(words);
        let sheet = div()
            .id("choose-sheet")
            .track_focus(&self.choose_focus)
            .on_key_down(cx.listener(Self::on_choose_key))
            .on_click(|_, _, cx| cx.stop_propagation())
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(look.length(token::RADIUS_LG) * 1.75)
            .bg(look.color(token::BG_OVERLAY))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .shadow(look.shadows(token::SHADOW_OVERLAY))
            .child(top)
            .child(body)
            .child(foot);
        Some(
            div()
                .id("choose-dim")
                .absolute()
                .inset_0()
                .occlude()
                .px(look.length(token::SPACE_4))
                .pt(look.length(token::SPACE_4))
                .pb(look.length(token::SPACE_3))
                .bg(look.color(token::BG_BASE).opacity(0.72))
                .on_click(cx.listener(|this, _, window, cx| this.close_choose(window, cx)))
                .child(labelled(sheet, Role::Dialog, question))
                .into_any_element(),
        )
    }
}

/// A layer over its parent that hands `then` the size the parent was laid out at.
fn measure(then: impl Fn(Size<Pixels>, &mut Window) + 'static) -> impl IntoElement {
    canvas(move |bounds, window, _| then(bounds.size, window), |_, _, _, _| {}).absolute().top_0().left_0().size_full()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_band_says_how_many_options_are_drawn_in_words() {
        assert_eq!(drawn_in_words(1), "One option drawn as a picture");
        assert_eq!(drawn_in_words(2), "Two options drawn as pictures");
        assert_eq!(drawn_in_words(3), "Three options drawn as pictures");
        assert_eq!(drawn_in_words(12), "12 options drawn as pictures");
    }

    #[test]
    fn left_and_right_light_the_next_option_and_wrap_at_either_end() {
        assert_eq!(step_lit(0, 1, 3), 1);
        assert_eq!(step_lit(2, 1, 3), 0, "→ on the last lights the first");
        assert_eq!(step_lit(0, -1, 3), 2, "← on the first lights the last");
        assert_eq!(step_lit(0, 1, 0), 0);
    }

    #[test]
    fn a_number_key_chooses_that_option_and_one_past_the_last_chooses_nothing() {
        assert_eq!(chosen_by_key("1", 4), Some(0));
        assert_eq!(chosen_by_key("4", 4), Some(3));
        assert_eq!(chosen_by_key("5", 4), None);
        assert_eq!(chosen_by_key("0", 4), None);
        assert_eq!(chosen_by_key("a", 4), None);
    }

    #[test]
    fn stills_that_do_not_fit_side_by_side_show_a_run_that_holds_the_lit_one() {
        assert_eq!(shown_run(Some(0), 3, 3), 0..3, "all fit");
        assert_eq!(shown_run(Some(0), 3, 1), 0..1);
        assert_eq!(shown_run(Some(2), 3, 1), 2..3, "one at a time follows the lit one");
        assert_eq!(shown_run(Some(2), 3, 2), 1..3);
        assert_eq!(shown_run(None, 3, 2), 0..2, "an option in words lit keeps the first run");
        assert_eq!(how_many_fit(560., 168., 12.), 3);
        assert_eq!(how_many_fit(230., 168., 12.), 1);
        assert_eq!(how_many_fit(0., 168., 12.), 1, "never fewer than one");
    }

    #[test]
    fn a_still_takes_its_columns_width_unless_the_height_left_is_less() {
        assert_eq!(fit(760, 380, 380., 400.), Some((380., 190.)), "the column's width decides");
        assert_eq!(fit(760, 380, 1520., 2000.), Some((1520., 760.)), "and a still is drawn past its own size to fill it");
        let (w, h) = fit(760, 2000, 380., 400.).unwrap();
        assert!((h - 400.).abs() < 1e-3 && (w - 152.).abs() < 1e-3, "a tall still: the height left decides, {w} by {h}");
        assert_eq!(fit(760, 380, 0., 0.), None, "a place not yet laid out draws nothing");
    }
}
