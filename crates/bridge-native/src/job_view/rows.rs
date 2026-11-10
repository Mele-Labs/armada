//! What the thread draws for each item Fleet serves (`job-view.md`, A0 to A7). Spacious take: the
//! words in one centred column, each row a quiet line or a card, folded where Electron folds.
//!
//! This file holds the frame, the messages and the shared pieces; `lines.rs` holds the other rows.
//! Cards that put something to the person (plan, sketch, document, draft, ask, judged) are the next
//! slice and draw as one row naming their kind, in their place in the order.

use chrono::{DateTime, Local};
use gpui::{
    div, prelude::*, relative, svg, Animation, AnimationExt, AnyElement, ClipboardItem, Context, ElementId, FontWeight, Hsla,
    Role, SharedString, Transformation, Window,
};
use gpui_component::text::TextView;
use ipc::{PlanCardState, SessionCheckState, SessionRow, SessionVoice, ThreadItem};

use super::{Extra, JobView, Page};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};

/// The widest the words go, in units of the largest space token (960 px at the default spacing).
const COLUMN: f32 = 20.;

/// A time in the owner's clock, with the day when it is not today (`Stamp`, A1.11).
fn clock(at: &str, now: DateTime<Local>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&Local);
    let time = then.format("%H:%M:%S").to_string();
    Some(if then.date_naive() == now.date_naive() { time } else { format!("{} {time}", then.format("%b %-d")) })
}

/// A time as how long ago it was: `<1m ago`, `5m ago`, `3h ago`, `2d ago`.
fn age(at: &str, now: DateTime<Local>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(at).ok()?;
    let minutes = (now.with_timezone(&chrono::Utc) - then.with_timezone(&chrono::Utc)).num_minutes();
    Some(match minutes {
        ..=0 => "<1m ago".to_string(),
        1..=59 => format!("{minutes}m ago"),
        60..=1439 => format!("{}h ago", minutes / 60),
        _ => format!("{}d ago", minutes / 1440),
    })
}

/// How long a thing ran: `7s`, `1m 05s`, `1h 02m`.
pub(super) fn elapsed_words(from: &str, to: &str) -> Option<String> {
    let from = DateTime::parse_from_rfc3339(from).ok()?;
    let to = DateTime::parse_from_rfc3339(to).ok()?;
    let secs = (to - from).num_seconds().max(0);
    Some(match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {:02}s", secs / 60, secs % 60),
        _ => format!("{}h {:02}m", secs / 3600, secs % 3600 / 60),
    })
}

/// What a person's message says when it answers or approves instead of talking (`Did`, A2.4).
#[derive(Debug, PartialEq)]
enum Act<'a> {
    Approved(&'a str),
    Answered { target: &'a str, said: &'a str },
}

const CLOSING: [&str; 2] = [
    "Fleet took this off your waiting list.",
    "This settles the item, so Fleet took it off your waiting list. Do not add it back.",
];

fn act_of(text: &str) -> Option<Act<'_>> {
    if let Some(address) = text.strip_prefix("Approved: ") {
        if !address.is_empty() && !address.contains(char::is_whitespace) {
            return Some(Act::Approved(address));
        }
    }
    let rest = text.strip_prefix("Re: ")?;
    let (target, said) = rest.split_once('\n').unwrap_or((rest, ""));
    let mut said = said.trim();
    for closing in CLOSING {
        said = said.strip_suffix(closing).map_or(said, str::trim_end);
    }
    Some(Act::Answered { target, said })
}

/// The address a walk approval shows: the query that names the walk, else the whole address.
fn shown_target(address: &str) -> &str {
    address.find("?walk=").or_else(|| address.find("&walk=")).map_or(address, |at| &address[at..])
}

pub(super) fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// The glyph sizes the thread draws at.
#[derive(Clone, Copy)]
pub(super) enum Size {
    /// 12 px.
    Small,
    /// 14 px.
    Large,
    /// 16 px.
    Mark,
}

impl Size {
    fn of(self, look: &Look) -> gpui::Pixels {
        match self {
            Size::Small => look.length(token::SPACE_3),
            Size::Large => look.length(token::SPACE_4) - look.length(token::BORDER_WIDTH) * 2.,
            Size::Mark => look.length(token::SPACE_4),
        }
    }
}

pub(super) fn glyph(look: &Look, name: &'static str, size: Size, ink: Hsla) -> gpui::Svg {
    svg().path(icon_path(name)).flex_none().size(size.of(look)).text_color(ink)
}

pub(super) fn tip(said: impl Into<SharedString>) -> impl Fn(&mut Window, &mut gpui::App) -> gpui::AnyView {
    let said: SharedString = said.into();
    move |window, cx| gpui_component::tooltip::Tooltip::new(said.clone()).build(window, cx)
}

pub(super) fn eyebrow(look: &Look, said: &str) -> gpui::Div {
    div()
        .flex_none()
        .text_size(look.length(token::TEXT_2XS))
        .line_height(look.length(token::LEADING_2XS))
        .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
        .child(said.to_uppercase())
}

pub(super) fn check_look(state: SessionCheckState) -> (&'static str, Token, &'static str) {
    match state {
        SessionCheckState::Running => ("circle-dot", token::STEP_RUNNING, "running"),
        SessionCheckState::Passed => ("check", token::STEP_ADVANCED, "passed"),
        SessionCheckState::Failed => ("x", token::STEP_FAILED, "failed"),
        SessionCheckState::TimedOut => ("x", token::STEP_FAILED, "timed out"),
    }
}

impl JobView {
    /// One element of the shown Job's list: an item, or what stands after the items.
    pub(super) fn render_entry(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let nothing = || div().h(gpui::px(0.)).into_any_element();
        let Some(id) = self.shown.clone() else { return nothing() };
        let (Some(page), Some(held)) = (self.pages.get(&id), self.thread(&id, cx)) else { return nothing() };
        let look = cx.look();
        let working = self.working(&id, cx);
        let now = Local::now();
        if page.lead == 1 && ix == 0 {
            return self.earlier_row(&id, look, cx);
        }
        let ix = ix - page.lead;
        let drawn = if ix < page.items {
            let Some(item) = held.items.get(ix) else { return nothing() };
            let last = ix + 1 == page.items;
            let asking = held.head.asked.is_some();
            self.item(ix, item, last && working && !asking, page, look, now, window, cx)
        } else {
            self.extra(&held.head, page, ix - page.items, look, now, cx)
        };
        let Some(drawn) = drawn else { return nothing() };
        div()
            .w_full()
            .flex()
            .justify_center()
            .px(look.length(token::SPACE_6))
            .pb(look.length(token::SPACE_6) + look.length(token::SPACE_2))
            .child(div().w_full().min_w_0().max_w(look.length(token::SPACE_12) * COLUMN).child(drawn))
            .into_any_element()
    }

    /// The row that opens the list when Fleet has older items: one press reads them (A1.6).
    fn earlier_row(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let reading = self.fleet.as_ref().is_some_and(|fleet| fleet.read(cx).reading_older(id));
        let job = id.clone();
        div()
            .w_full()
            .flex()
            .justify_center()
            .pb(look.length(token::SPACE_6))
            .child(labelled(
                super::acts::button(look, "show-earlier", "Show earlier", super::acts::Tone::Ghost, false, None, !reading)
                    .tooltip(tip("Show earlier"))
                    .on_click(cx.listener(move |this, _, _, cx| this.want_earlier(&job, cx))),
                Role::Button,
                "Show earlier",
            ))
            .into_any_element()
    }

    /// A time as the thread reads it now: the clock, or its age once a press has flipped every stamp.
    pub(super) fn stamp(&self, at: &ipc::Instant, look: &Look, now: DateTime<Local>, still: bool, cx: &Context<Self>) -> AnyElement {
        let (on_clock, ago) = (clock(at.as_str(), now), age(at.as_str(), now));
        let (shown, other) = if self.aged { (ago, on_clock) } else { (on_clock, ago) };
        let Some(shown) = shown else {
            // Not an instant: the text as it came, no button.
            return div().flex_none().child(at.as_str().to_string()).into_any_element();
        };
        let base = div()
            .id(SharedString::from(format!("stamp:{}", at.as_str())))
            .flex_none()
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_SUBTLE))
            .child(shown);
        if still {
            return base.into_any_element();
        }
        labelled(
            base.cursor_pointer()
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .on_click(cx.listener(|this, _, _, cx| {
                    cx.stop_propagation();
                    this.flip_stamps(cx)
                }))
                .tooltip(tip(other.unwrap_or_default())),
            Role::Button,
            "Time",
        )
        .into_any_element()
    }

    /// Markdown at the thread's body size (15 over 24).
    pub(super) fn prose(&self, key: String, text: &str, look: &Look) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        TextView::markdown(ElementId::Name(format!("md:{job}:{key}").into()), text.to_string())
            .w_full()
            .text_size(look.length(token::TEXT_BASE))
            .line_height(relative(1.6))
            .text_color(look.color(token::FG_DEFAULT))
            .into_any_element()
    }

    /// A disclosure: a line that opens a body under it on a press, and shuts it on the next. The
    /// open ones are kept in the page, so scrolling away and back finds them as they were.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fold(
        &self,
        ix: usize,
        key: String,
        label: String,
        head: Vec<AnyElement>,
        body: Option<AnyElement>,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let open = page.opened.contains(&key);
        let name = SharedString::from(key.clone());
        let line = labelled(
            div()
                .id(name)
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .w_full()
                .min_w_0()
                .cursor_pointer()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(if open { token::FG_DEFAULT } else { token::FG_MUTED }))
                .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
                .children(head)
                .on_click(cx.listener(move |this, _, _, cx| this.toggle(ix, key.clone(), cx))),
            Role::Button,
            label,
        );
        div()
            .flex()
            .flex_col()
            .min_w_0()
            .child(line)
            .children(body.filter(|_| open).map(|body| div().mt(look.length(token::SPACE_1)).pl(look.length(token::SPACE_4)).child(body)))
            .into_any_element()
    }

    /// A line that is not a press: its glyph, its words, its time.
    pub(super) fn plain(&self, id: String, label: String, look: &Look, parts: Vec<AnyElement>) -> AnyElement {
        labelled(
            div()
                .id(SharedString::from(id))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .min_w_0()
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_SM))
                .children(parts),
            Role::Label,
            label,
        )
        .into_any_element()
    }

    /// A row that opens another Job on a press.
    pub(super) fn opens(&self, job: &str, id: String, label: String, cx: &Context<Self>, row: gpui::Div) -> AnyElement {
        let job = job.to_string();
        labelled(
            row.id(SharedString::from(id)).cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.open_job(job.clone(), cx))),
            Role::Button,
            label,
        )
        .into_any_element()
    }

    /// The padding a bar or a bordered line takes.
    pub(super) fn padded(look: &Look, bar: gpui::Div) -> gpui::Div {
        bar.px(look.length(token::SPACE_3)).py(look.length(token::SPACE_2)).rounded(look.length(token::RADIUS_MD))
    }

    #[allow(clippy::too_many_arguments)]
    fn item(
        &self,
        ix: usize,
        item: &ThreadItem,
        live: bool,
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        Some(match item {
            ThreadItem::Row { row } => return self.row(ix, row, page, look, now, window, cx),
            ThreadItem::Calls { id, rows, .. } => self.calls(ix, id, rows, live, page, look, cx),
            ThreadItem::Subagent { id, subagent, .. } => self.subagent(id, subagent, look),
            ThreadItem::Checks { id, rows } => self.checks(ix, id, rows, page, look, now, cx),
            ThreadItem::FleetTurn { id, notice, rows, handled } => self.fleet_turn(ix, id, notice, rows, *handled, page, look, cx),
            ThreadItem::Sketch(card) => self.sketch(ix, card, page, look, now, cx),
            ThreadItem::File { id, name, .. } => self.file_row(id, name, look),
            ThreadItem::Document(card) => self.document(ix, card, page, look, now, cx),
            ThreadItem::Plan(plan) => self.plan_card(ix, plan, page, look, now, cx),
            ThreadItem::Judged { id, at, artifact, .. } => self.judged_row(id, &artifact.title, at, look, now, cx),
            ThreadItem::Change { id, at, what, from, to } => self.change(id, at, *what, from, to, look, now, cx),
            ThreadItem::NotStarted { id, at, cause, log } => self.not_started(ix, id, at, cause, log, page, look, now, cx),
        })
    }

    /// What stands after the items (A1.4).
    fn extra(&self, head: &ipc::ThreadHead, page: &Page, at: usize, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        Some(match page.extras.get(at)? {
            Extra::Plan(_) => self.plan_card(page.items + at, head.plan.as_ref()?, page, look, now, cx),
            Extra::Sketch(_) => self.sketch_pin(page.items + at, head.sketch.as_ref()?, page, look, cx),
            Extra::Pending(i) => {
                let one = page.pending.get(*i)?;
                self.you(&format!("pending:{}", one.key), &one.text, &[], None, true, false, look, now, cx)
            }
            Extra::Working => {
                let ink = look.color(token::STATUS_RUNNING);
                labelled(
                    div()
                        .id("working")
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_2))
                        .py(look.length(token::SPACE_1))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_SM))
                        .text_color(look.color(token::FG_MUTED))
                        .child(div().id("working-mark").child(self.working_mark(look, ink)).tooltip(tip("Working")))
                        .child("Thinking"),
                    Role::Status,
                    "Working",
                )
                .into_any_element()
            }
        })
    }

    /// The turning ring that says the agent is working.
    pub(super) fn working_mark(&self, look: &Look, ink: Hsla) -> AnyElement {
        svg()
            .path(icon_path("loader-circle"))
            .flex_none()
            .size(look.length(token::SPACE_4))
            .text_color(ink)
            .with_animation(
                "working-turn",
                Animation::new(look.duration(token::DURATION_SPIN)).repeat(),
                |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
            )
            .into_any_element()
    }

    /// A card the next slice draws whole: its kind, and what it is about, in its place.
    pub(super) fn card(&self, id: &str, kind: &str, about: Option<&String>, icon: &'static str, look: &Look) -> AnyElement {
        let said = match about {
            Some(about) => format!("{kind}: {about}"),
            None => kind.to_string(),
        };
        labelled(
            div()
                .id(SharedString::from(format!("card:{id}")))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .w_full()
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_3))
                .rounded(look.length(token::RADIUS_LG))
                .bg(look.color(token::FG_DEFAULT).opacity(0.04))
                .border(look.length(token::BORDER_WIDTH))
                .border_color(look.color(token::FG_DEFAULT).opacity(0.07))
                .text_size(look.length(token::TEXT_SM))
                .line_height(look.length(token::LEADING_SM))
                .text_color(look.color(token::FG_DEFAULT))
                .child(glyph(look, icon, Size::Mark, look.color(token::FG_MUTED)))
                .child(div().flex_1().min_w_0().truncate().child(said.clone())),
            Role::Row,
            said,
        )
        .into_any_element()
    }

    /// The person's words: a bubble at the right, its band under it (A2.1).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn you(
        &self,
        id: &str,
        text: &str,
        chips: &[String],
        at: Option<&ipc::Instant>,
        sending: bool,
        queued: bool,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let band = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .text_color(look.color(token::FG_SUBTLE))
            .when(sending, |band| {
                band.child(
                    div()
                        .id(SharedString::from(format!("sending:{id}")))
                        .child(
                            svg()
                                .path(icon_path("loader-circle"))
                                .flex_none()
                                .size(look.length(token::SPACE_3))
                                .text_color(look.color(token::FG_SUBTLE))
                                .with_animation(
                                    SharedString::from(format!("sending-turn:{id}")),
                                    Animation::new(look.duration(token::DURATION_PULSE)).repeat(),
                                    |mark, delta| mark.with_transformation(Transformation::rotate(gpui::percentage(delta))),
                                ),
                        )
                        .tooltip(tip("Sending")),
                )
            })
            .when(queued, |band| {
                band.child(
                    div()
                        .id(SharedString::from(format!("queued:{id}")))
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_1))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child(glyph(look, "clock", Size::Small, look.color(token::FG_MUTED)))
                        .child("Queued")
                        .tooltip(tip("Queued until the subagent returns")),
                )
            })
            .children(at.map(|at| self.stamp(at, look, now, false, cx)));
        labelled(
            div()
                .id(SharedString::from(format!("you:{id}")))
                .flex()
                .flex_col()
                .items_end()
                .when(sending, |row| row.opacity(0.6))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap(look.length(token::SPACE_2))
                        .max_w(relative(0.8))
                        .child(
                            div()
                                .px(look.length(token::SPACE_4) + look.length(token::SPACE_1))
                                .py(look.length(token::SPACE_3))
                                .rounded(look.length(token::RADIUS_LG))
                                .bg(look.color(token::FG_DEFAULT).opacity(0.07))
                                .child(self.prose(format!("you:{id}"), text, look)),
                        )
                        .when(!chips.is_empty(), |column| {
                            column.child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .justify_end()
                                    .gap(look.length(token::SPACE_2))
                                    .children(chips.iter().map(|chip| self.chip(chip, look))),
                            )
                        })
                        .child(band),
                ),
            Role::Region,
            "Your message",
        )
        .into_any_element()
    }

    /// A name drawn as a read-only chip: a file, or a tag the words did not place.
    fn chip(&self, said: &str, look: &Look) -> gpui::Div {
        div()
            .flex_none()
            .max_w_full()
            .truncate()
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .child(said.to_string())
    }

    /// The agent's words, with the bar under them that shows on hover (A2.3).
    fn agent(&self, id: &str, text: &str, at: &ipc::Instant, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let group = SharedString::from(format!("agent:{id}"));
        let button = |name: &'static str, icon: &'static str| {
            div()
                .id(SharedString::from(format!("{name}:{id}")))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .size(look.length(token::H_CONTROL))
                .rounded_full()
                .cursor_pointer()
                .hover(|style| style.bg(look.color(token::BG_HOVER)))
                .child(glyph(look, icon, Size::Large, look.color(token::FG_SUBTLE)))
                .tooltip(tip(name))
        };
        let said = text.to_string();
        let copy = labelled(
            button("Copy message", "copy").on_click(cx.listener({
                let said = said.clone();
                move |_, _, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(said.clone()))
            })),
            Role::Button,
            "Copy message",
        );
        let reply = labelled(
            button("Reply", "reply").on_click(cx.listener({
                let said = said.clone();
                move |this, _, window, cx| this.quote(&said, window, cx)
            })),
            Role::Button,
            "Reply",
        );
        let bar = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .ml(-look.length(token::SPACE_2))
            .opacity(0.)
            .group_hover(group.clone(), |style| style.opacity(1.))
            .child(copy)
            .child(reply)
            .child(div().ml(look.length(token::SPACE_2)).child(self.stamp(at, look, now, false, cx)));
        div()
            .group(group)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .child(self.prose(format!("agent:{id}"), text, look))
            .child(bar)
            .into_any_element()
    }

    /// The person's act: an approval or an answer, a filled bar (A2.4).
    fn did(&self, id: &str, act: Act<'_>, at: &ipc::Instant, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> AnyElement {
        let (word, label, icon, target, said) = match act {
            Act::Approved(address) => ("Approved", "You approved", "check", shown_target(address).to_string(), ""),
            Act::Answered { target, said } => ("Answered", "You answered", "reply", target.to_string(), said),
        };
        let ink = look.color(token::FG_INVERSE);
        let chip = div()
            .id(SharedString::from(format!("did-chip:{id}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .min_w_0()
            .px(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(ink)
            .font_family(mono(look))
            .text_size(look.length(token::TEXT_2XS))
            .line_height(look.length(token::LEADING_XS))
            .child(glyph(look, icon, Size::Small, ink))
            .child(div().min_w_0().truncate().child(target.clone()))
            .when(target.len() > 40, |chip| chip.tooltip(tip(target.clone())));
        labelled(
            div()
                .id(SharedString::from(format!("did:{id}")))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .px(look.length(token::SPACE_3))
                .py(look.length(token::SPACE_2))
                .rounded(look.length(token::RADIUS_MD))
                .bg(look.color(token::FG_DEFAULT))
                .text_color(ink)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(look.length(token::SPACE_3))
                        .child(eyebrow(look, word))
                        .child(div().flex_1().min_w_0().flex().child(chip))
                        .child(div().text_color(ink).child(self.stamp(at, look, now, true, cx))),
                )
                .when(!said.is_empty(), |bar| {
                    bar.child(
                        div()
                            .text_size(look.length(token::TEXT_SM))
                            .line_height(look.length(token::LEADING_SM))
                            .child(said.to_string()),
                    )
                }),
            Role::Region,
            label,
        )
        .into_any_element()
    }

    /// The messages, from the person, the agent and another Job (A2).
    pub(super) fn message(&self, row: &SessionRow, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        Some(match row {
            SessionRow::Message { id, at, from: SessionVoice::You, text, files, tags, queued, .. } => {
                if let Some(act) = act_of(text) {
                    return Some(self.did(id, act, at, look, now, cx));
                }
                let chips: Vec<String> =
                    files.iter().map(|file| file.name.clone()).chain(tags.iter().map(|tag| tag.title.clone())).collect();
                self.you(id, text, &chips, Some(at), false, *queued, look, now, cx)
            }
            SessionRow::Message { id, at, from: SessionVoice::Agent, text, .. } => self.agent(id, text, at, look, now, cx),
            SessionRow::Message { id, at, from: SessionVoice::Session { id: sender, title }, text, .. } => {
                let band = div()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_2))
                    .w_full()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_1))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_size(look.length(token::TEXT_XS))
                    .line_height(look.length(token::LEADING_SM))
                    .text_color(look.color(token::FG_DEFAULT))
                    .child(glyph(look, "square-terminal", Size::Small, look.color(token::FG_MUTED)))
                    .child(
                        div()
                            .flex_none()
                            .font_family(mono(look))
                            .text_size(look.length(token::TEXT_2XS))
                            .text_color(look.color(token::FG_MUTED))
                            .child(sender.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .child(title.clone()),
                    )
                    .child(self.stamp(at, look, now, true, cx));
                let band = if sender == "fleet" {
                    band.into_any_element()
                } else {
                    self.opens(sender, format!("from:{id}"), format!("Open Job {title}"), cx, band)
                };
                labelled(
                    div()
                        .id(SharedString::from(format!("from-card:{id}")))
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .rounded(look.length(token::RADIUS_MD))
                        .border(look.length(token::BORDER_WIDTH))
                        .border_color(look.color(token::ACCENT))
                        .bg(look.color(token::BG_RAISED))
                        .child(band)
                        .child(
                            div()
                                .px(look.length(token::SPACE_3))
                                .py(look.length(token::SPACE_2))
                                .child(self.prose(format!("from:{id}"), text, look)),
                        ),
                    Role::Region,
                    format!("Message from {title}"),
                )
                .into_any_element()
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<Local> {
        DateTime::parse_from_rfc3339(text).expect("an instant").with_timezone(&Local)
    }

    #[test]
    fn an_age_reads_in_the_largest_whole_unit() {
        let now = at("2026-11-09T12:00:00Z");
        assert_eq!(age("2026-11-09T11:59:40Z", now).as_deref(), Some("<1m ago"));
        assert_eq!(age("2026-11-09T11:55:00Z", now).as_deref(), Some("5m ago"));
        assert_eq!(age("2026-11-09T09:00:00Z", now).as_deref(), Some("3h ago"));
        assert_eq!(age("2026-11-07T11:00:00Z", now).as_deref(), Some("2d ago"));
        assert_eq!(age("2026-11-09T12:30:00Z", now).as_deref(), Some("<1m ago"), "ahead of now reads as under a minute");
        assert_eq!(age("not a time", now), None);
    }

    #[test]
    fn a_run_reads_in_seconds_then_minutes_then_hours() {
        assert_eq!(elapsed_words("2026-11-09T10:00:00Z", "2026-11-09T10:00:07Z").as_deref(), Some("7s"));
        assert_eq!(elapsed_words("2026-11-09T10:00:00Z", "2026-11-09T10:01:05Z").as_deref(), Some("1m 05s"));
        assert_eq!(elapsed_words("2026-11-09T10:00:00Z", "2026-11-09T11:02:00Z").as_deref(), Some("1h 02m"));
    }

    #[test]
    fn an_approval_and_an_answer_are_acts_and_other_words_are_not() {
        assert_eq!(act_of("Approved: http://localhost:5173/?walk=abc"), Some(Act::Approved("http://localhost:5173/?walk=abc")));
        assert_eq!(act_of("Approved: two words"), None);
        assert_eq!(
            act_of("Re: Which branch?\nThe main one.\nFleet took this off your waiting list."),
            Some(Act::Answered { target: "Which branch?", said: "The main one." })
        );
        assert_eq!(act_of("Re: Which branch?"), Some(Act::Answered { target: "Which branch?", said: "" }));
        assert_eq!(act_of("Please go on"), None);
        assert_eq!(shown_target("http://localhost:5173/page?walk=abc"), "?walk=abc");
        assert_eq!(shown_target("http://example.com/x"), "http://example.com/x");
    }
}
