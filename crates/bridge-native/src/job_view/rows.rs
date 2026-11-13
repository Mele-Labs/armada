//! What the thread draws for each item Fleet serves (`job-view.md`, A0 to A7). Spacious take: the
//! words in one centred column, each row a quiet line or a card, folded where Electron folds.
//!
//! This file holds the frame, the messages and the shared pieces; `lines.rs` holds the other rows.
//! Cards that put something to the person (plan, sketch, document, draft, ask, judged) are the next
//! slice and draw as one row naming their kind, in their place in the order.

use chrono::{DateTime, Local};
use gpui::{
    canvas, div, prelude::*, relative, svg, Animation, AnimationExt, AnyElement, App, Context, ElementId, FontWeight, Hsla,
    Role, SharedString, Transformation, Window,
};
use gpui_component::text::{markdown_ast, InlineElement, InlineRenderContext, MarkdownExtensions, MarkdownNode, MarkdownParseContext, MarkdownPlugin, TextView};
use ipc::{SessionCheckState, SessionRow, SessionTag, SessionVoice, TagKind, ThreadItem};

use std::cell::Cell;
use std::rc::Rc;

use super::acts::refusal;
use super::menus::notice_entries;
use super::quick::{Spacing, TextSize};
use super::{Extra, JobView, Page};
use crate::assets::icon_path;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};

/// The widest the words go, in units of the largest space token (960 px at the default spacing).

/// A time in the owner's clock, with the day when it is not today (`Stamp`, A1.11).
pub(super) fn clock(at: &str, now: DateTime<Local>) -> Option<String> {
    let then = DateTime::parse_from_rfc3339(at).ok()?.with_timezone(&Local);
    Some(crate::settings::time::stamp_of(then, crate::settings::time::held(), now))
}

/// A time as how long ago it was: `<1m ago`, `5m ago`, `3h ago`, `2d ago`.
pub(crate) fn age(at: &str, now: DateTime<Local>) -> Option<String> {
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

/// A lone line break kept as a break, as Electron's Prose draws it (`remark-breaks`): each line that
/// another line follows in the same paragraph ends in a hard break, outside fenced code.
pub(super) fn with_breaks(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut fenced = false;
    let mut out = String::with_capacity(text.len() + lines.len() * 2);
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence {
            fenced = !fenced;
        }
        out.push_str(line);
        let Some(next) = lines.get(at + 1) else { break };
        let opens_fence = next.trim_start().starts_with("```") || next.trim_start().starts_with("~~~");
        if !fenced && !fence && !line.trim().is_empty() && !next.trim().is_empty() && !opens_fence && !line.ends_with("  ") && !line.ends_with('\\') {
            out.push_str("  ");
        }
        out.push('\n');
    }
    out
}

/// When an item began, where it says: a run's end is the next item's start.
pub(super) fn began(item: &ThreadItem) -> Option<&ipc::Instant> {
    match item {
        ThreadItem::Row { row } => Some(row.at()),
        ThreadItem::Calls { rows, .. } | ThreadItem::Checks { rows, .. } => rows.first().map(SessionRow::at),
        ThreadItem::FleetTurn { notice, .. } => Some(notice.at()),
        ThreadItem::Subagent { at, .. }
        | ThreadItem::File { at, .. }
        | ThreadItem::Change { at, .. }
        | ThreadItem::NotStarted { at, .. }
        | ThreadItem::Judged { at, .. } => Some(at),
        ThreadItem::Sketch(_) | ThreadItem::Document(_) | ThreadItem::Plan(_) => None,
    }
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

/// Whether his words are an approval or an answer, drawn as an act rather than a bubble.
pub(super) fn is_act(text: &str) -> bool {
    act_of(text).is_some()
}

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

pub(crate) fn mono(look: &Look) -> SharedString {
    look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()
}

/// The glyph sizes the thread draws at.
#[derive(Clone, Copy)]
pub(crate) enum Size {
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

pub(crate) fn glyph(look: &Look, name: &'static str, size: Size, ink: Hsla) -> gpui::Svg {
    svg().path(icon_path(name)).flex_none().size(size.of(look)).text_color(ink)
}

/// The mark at the head of a fold: right while shut, down while open, as the reasoning row draws it.
pub(super) fn chevron(look: &Look, open: bool) -> AnyElement {
    glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_MUTED)).into_any_element()
}

pub(crate) fn tip(said: impl Into<SharedString>) -> impl Fn(&mut Window, &mut gpui::App) -> gpui::AnyView {
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
    /// Aa's Spacing: the gap under each row of the thread, the Show earlier row among them.
    pub(super) fn row_gap(&self, look: &Look) -> gpui::Pixels {
        match self.composer.quick.thread().spacing {
            Spacing::Compact => look.length(token::SPACE_4),
            Spacing::Default => look.length(token::SPACE_6) + look.length(token::SPACE_2),
            Spacing::Roomy => look.length(token::SPACE_8) + look.length(token::SPACE_2),
        }
    }

    /// One element of the shown Job's list: an item, or what stands after the items.
    pub(super) fn render_entry(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let nothing = || div().h(gpui::px(0.)).into_any_element();
        let Some(id) = self.shown.clone() else { return nothing() };
        // A picture his message carried is read the first time its row is drawn.
        let wanted = match (self.pages.get(&id), self.thread(&id, cx)) {
            (Some(page), Some(held)) => match ix.checked_sub(page.lead + page.before).and_then(|at| held.items.get(at)) {
                Some(ThreadItem::Row { row: SessionRow::Message { files, .. } | SessionRow::Sent { files, .. } })
                    if files.iter().any(super::pictures::is_picture) =>
                {
                    Some((files.clone(), false))
                }
                // And a picture a `Read` call looked at, the first time its run is drawn.
                Some(ThreadItem::Calls { rows, .. }) => {
                    let read: Vec<ipc::SentFile> = rows
                        .iter()
                        .filter_map(|row| match row {
                            SessionRow::Tool { image: Some(path), .. } => super::pictures::read_picture(path),
                            _ => None,
                        })
                        .collect();
                    (!read.is_empty()).then_some((read, true))
                }
                _ => None,
            },
            _ => None,
        };
        match wanted {
            Some((files, false)) => self.want_pictures(&id, &files, window, cx),
            Some((files, true)) => self.want_read_pictures(&id, &files, window, cx),
            None => {}
        }
        let (Some(page), Some(held)) = (self.pages.get(&id), self.thread(&id, cx)) else { return nothing() };
        let look = cx.look();
        let working = self.working(&id, cx);
        let now = Local::now();
        if ix < page.lead {
            // The row a move put at the head comes after Show earlier.
            let moved_from = self.moved(&id, "forked_from", cx);
            return match moved_from {
                Some(moved) if ix + 1 == page.lead => div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .px(look.length(token::SPACE_4) + super::bleed(look))
                    .pb(self.row_gap(look))
                    .child(div().w_full().min_w_0().max_w(self.column(look)).child(self.moved_row(&id, "from", moved, look, cx)))
                    .into_any_element(),
                _ => self.earlier_row(&id, look, cx),
            };
        }
        // The Job this one took the place of: its items, then a plain rule, read only.
        if ix - page.lead < page.before {
            return self.earlier_entry(&id, ix - page.lead, look, now, window, cx);
        }
        let ix = ix - page.lead - page.before;
        let drawn = if ix < page.items {
            let Some(item) = held.items.get(ix) else { return nothing() };
            let last = ix + 1 == page.items;
            let asking = held.head.asked.is_some();
            // A finished run's last call ended when the item after it began.
            let next = held.items.get(ix + 1).and_then(began);
            match item {
                ThreadItem::Row { row: SessionRow::Reasoning { id, text, .. } } => {
                    let (seconds, open) = super::lines::reasoning_reading(&held.items, ix);
                    Some(self.reasoning(ix, id, text, seconds, open, page, look, cx))
                }
                _ => self.item(ix, item, last && working && !asking, next, page, look, now, window, cx),
            }
        } else {
            self.extra(&held.head, &held.items, page, ix - page.items, look, now, cx)
        };
        let Some(drawn) = drawn else { return nothing() };
        // Aa's Spacing: the gap under each row, and the room a message takes above itself and above the row after it.
        let spacing = self.composer.quick.thread().spacing;
        let gap = self.row_gap(look);
        let room = match spacing {
            Spacing::Compact => look.length(token::SPACE_1),
            Spacing::Default => look.length(token::SPACE_2),
            Spacing::Roomy => look.length(token::SPACE_4),
        };
        let is_message = |at: usize| matches!(held.items.get(at), Some(ThreadItem::Row { row: SessionRow::Message { .. } }));
        let roomy = ix > 0 && ix < page.items && (is_message(ix) || is_message(ix - 1));
        let row = div()
            .id(SharedString::from(format!("entry:{id}:{ix}")))
            .w_full()
            .flex()
            .justify_center()
            .px(look.length(token::SPACE_4) + super::bleed(look))
            .pb(gap)
            .when(roomy, |row| row.pt(room))
            .child(div().w_full().min_w_0().max_w(self.column(look)).child(drawn));
        // The person's own words are drawn whole the frame they are sent: a fade from nothing here made a
        // send look like it drew nothing, and made Fleet's echo of a pending row blink out and back.
        let own = match held.items.get(ix) {
            Some(ThreadItem::Row { row: SessionRow::Message { from: SessionVoice::You, .. } }) => true,
            Some(ThreadItem::Document(card)) => page.made_pages.contains(&card.page),
            _ => false,
        };
        if !page.arrived.contains(&ix) || own {
            return row.into_any_element();
        }
        // A row that came in after the first draw rises from `space-2` below, from nothing (A1.8).
        let rise = look.length(token::SPACE_2);
        row.relative()
            .with_animation(
                SharedString::from(format!("arrive:{id}:{ix}")),
                Animation::new(look.duration(token::DURATION_ENTER)).with_easing(gpui::ease_out_quint()),
                move |row, delta| row.opacity(delta).top(rise * (1. - delta)),
            )
            .into_any_element()
    }

    /// An element of the Job the shown one took the place of: one of its items, or the plain rule
    /// under the last of them, as Electron's `replaces` divider draws it.
    #[allow(clippy::too_many_arguments)]
    fn earlier_entry(&self, id: &SharedString, at: usize, look: &Look, now: DateTime<Local>, window: &mut Window, cx: &Context<Self>) -> AnyElement {
        let nothing = || div().h(gpui::px(0.)).into_any_element();
        let (Some(page), Some(earlier)) = (self.pages.get(id), self.earlier_of(id, cx)) else { return nothing() };
        let Some(held) = self.thread(&earlier, cx) else { return nothing() };
        let column = |child: AnyElement| {
            div()
                .w_full()
                .flex()
                .justify_center()
                .px(look.length(token::SPACE_4) + super::bleed(look))
                .child(div().w_full().min_w_0().max_w(self.column(look)).child(child))
        };
        let Some(item) = held.items.get(at) else {
            let rule = div().id(SharedString::from(format!("handed-on:{id}"))).h(gpui::px(0.)).my(look.length(token::SPACE_4)).border_t(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE));
            return column(labelled(rule, Role::Splitter, "Handed on").into_any_element()).into_any_element();
        };
        // Its own index runs below the shown Job's, so a fold it opens measures this element again.
        let ix = at.wrapping_sub(page.before);
        let next = held.items.get(at + 1).and_then(began);
        let Some(drawn) = self.item(ix, item, false, next, page, look, now, window, cx) else { return nothing() };
        column(drawn).pb(self.row_gap(look)).into_any_element()
    }

    /// The row that opens the list when Fleet has older items: one press reads them (A1.6).
    fn earlier_row(&self, id: &SharedString, look: &Look, cx: &Context<Self>) -> AnyElement {
        let older = self.older_of(id, cx);
        let reading = self.fleet.as_ref().is_some_and(|fleet| fleet.read(cx).reading_older(&older));
        let job = id.clone();
        div()
            .w_full()
            .flex()
            .justify_center()
            .pb(self.row_gap(look))
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
        // Aa's Show times: no stamp is drawn, whatever a press has flipped them to.
        if !self.composer.quick.thread().show_times {
            return gpui::Empty.into_any_element();
        }
        self.stamp_sized(at, look, now, still, (token::TEXT_XS, token::LEADING_XS), cx)
    }

    /// A stamp outside the thread, whatever Aa's Show times says, in the size of the line it sits in: a gate's
    /// closing time in the plan's stretches (`Stamp`).
    pub(super) fn stamp_sized(&self, at: &ipc::Instant, look: &Look, now: DateTime<Local>, still: bool, (size, leading): (Token, Token), cx: &Context<Self>) -> AnyElement {
        let (on_clock, ago) = (clock(at.as_str(), now), age(at.as_str(), now));
        let (shown, other) = if self.aged { (ago, on_clock) } else { (on_clock, ago) };
        let Some(shown) = shown else {
            // Not an instant: the text as it came, no button.
            return div().flex_none().child(at.as_str().to_string()).into_any_element();
        };
        let base = div()
            .id(SharedString::from(format!("stamp:{}", at.as_str())))
            .flex_none()
            .text_size(look.length(size))
            .line_height(look.length(leading))
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

    /// Markdown at the thread's body size: 15 over 24 unless Aa's Text size says another.
    pub(super) fn prose(&self, key: String, text: &str, look: &Look, cx: &Context<Self>) -> AnyElement {
        self.prose_in(key, text, token::FG_DEFAULT, look, cx)
    }

    /// Prose in `colour`: a fold's body reads muted, as Electron's `armada-session-fold__body` has it.
    pub(super) fn prose_in(&self, key: String, text: &str, colour: crate::look::Token, look: &Look, cx: &Context<Self>) -> AnyElement {
        self.prose_tagged(key, text, colour, &[], look, cx)
    }

    /// [`Self::prose_in`] with `tags` drawn where his words wrote them (`placeTags`): each an `InlineTag` chip held
    /// in the line as one object, so the words around it stay selectable and a selection copies `@title`.
    pub(super) fn prose_tagged(&self, key: String, text: &str, colour: crate::look::Token, tags: &[SessionTag], look: &Look, cx: &Context<Self>) -> AnyElement {
        let job = self.shown.clone().unwrap_or_default();
        // A Job the words name is a link to it, as Electron's `SessionRefs` draws a thread's words.
        let text = super::refs::Refs::of(self.list.read(cx).records()).linked(text);
        let view = cx.weak_entity();
        let size = match self.composer.quick.thread().text_size {
            TextSize::Xs => token::TEXT_XS,
            TextSize::Sm => token::TEXT_SM,
            TextSize::Base => token::TEXT_BASE,
            TextSize::Lg => token::TEXT_LG,
        };
        let source = with_breaks(&text);
        // A block that runs takes Run beside Copy, and its words stop short of both.
        let acts = if source.lines().any(|line| ["```bash", "```sh", "```zsh", "```shell", "```console"].contains(&line.trim())) { 2. } else { 1. };
        let words = |id: String, source: String, base: usize| TextView::markdown(ElementId::Name(id.into()), source)
            .w_full()
            .text_size(look.length(size))
            .line_height(relative(1.6))
            .text_color(look.color(colour))
            // `.armada-prose__quote`: a faint accent box, an accent bar on its left, its right corners rounded.
            .blockquote_style(
                gpui::StyleRefinement::default()
                    .px(look.length(token::SPACE_3))
                    .py(look.length(token::SPACE_2))
                    .border_l(look.length(token::BORDER_WIDTH) * 2.)
                    .border_color(look.color(token::ACCENT))
                    .rounded_r(look.length(token::RADIUS_SM))
                    .bg(look.color(token::ACCENT_FAINT))
                    .text_color(look.color(token::FG_DEFAULT)),
            )
            .selectable(true)
            // `.armada-prose__block[data-acts="1"]`: the block's words stop short of the Copy over its corner.
            .style(gpui_component::text::TextViewStyle {
                code_block: gpui::StyleRefinement::default().pr(look.length(token::H_CONTROL_SM) * acts + look.length(token::SPACE_1) * (acts - 1.) + look.length(token::SPACE_2)),
                ..Default::default()
            })
            .code_block_actions(code_copy(look, job.to_string(), key.clone(), base))
            .when(!tags.is_empty(), |words| words.markdown_extensions(tag_extensions(tags, look)))
            // No surface navigates (design-system.md, hard rule 2): a Job opens here, an address in the browser.
            .on_link_click({
                let view = view.clone();
                move |address, _, _, cx| match super::refs::Refs::job_of(address) {
                    Some(id) => {
                        view.update(cx, |this, cx| this.open_job(id, cx)).ok();
                    }
                    None => cx.open_url(address),
                }
            });
        // A run a block of these words started is drawn right under that block (`InlineTerminal`): the words part there.
        let ran = crate::terminal::run::in_thread(&job, cx)
            .filter(|(block, end, _)| block.rsplit_once(crate::terminal::run::AT).is_some_and(|(of, _)| of == key) && *end <= source.len() && source.is_char_boundary(*end));
        let Some((_, end, run)) = ran else {
            return div().w_full().group(PROSE_GROUP).child(words(format!("md:{job}:{key}"), source, 0)).into_any_element();
        };
        let after = source[end..].trim_start_matches('\n').to_string();
        let gap = source.len() - end - after.len();
        div()
            .w_full()
            .group(PROSE_GROUP)
            .flex()
            .flex_col()
            .child(words(format!("md:{job}:{key}"), source[..end].to_string(), 0))
            .child(div().mt(look.length(token::SPACE_3)).child(run))
            .when(!after.trim().is_empty(), |line| line.child(div().mt(look.length(token::SPACE_3)).child(words(format!("md:{job}:{key}:after"), after, end + gap))))
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
        self.fold_from(ix, key, label, head, body, false, page, look, cx)
    }

    /// A fold that may start open: then a press shuts it, and the page keeps that it was pressed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fold_from(
        &self,
        ix: usize,
        key: String,
        label: String,
        head: Vec<AnyElement>,
        body: Option<AnyElement>,
        first_open: bool,
        page: &Page,
        look: &Look,
        cx: &Context<Self>,
    ) -> AnyElement {
        let open = first_open != page.opened.contains(&key);
        let name = SharedString::from(key.clone());
        let spot = page.keys.spot(&key, cx);
        let line = labelled(
            div()
                .id(name)
                .track_focus(&spot.focus)
                .focus_visible(|style| style.bg(look.color(token::BG_HOVER)).text_color(look.color(token::FG_DEFAULT)))
                .rounded(look.length(token::RADIUS_SM))
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
    pub(super) fn item(
        &self,
        ix: usize,
        item: &ThreadItem,
        live: bool,
        next: Option<&ipc::Instant>,
        page: &Page,
        look: &Look,
        now: DateTime<Local>,
        window: &mut Window,
        cx: &Context<Self>,
    ) -> Option<AnyElement> {
        Some(match item {
            ThreadItem::Row { row } => return self.row(ix, row, page, look, now, window, cx),
            ThreadItem::Calls { id, rows, .. } => self.calls(ix, id, rows, live, next, page, look, cx),
            ThreadItem::Subagent { id, subagent, .. } => self.subagent(id, subagent, look, cx),
            ThreadItem::Checks { id, rows } => self.checks(ix, id, rows, page, look, now, cx),
            ThreadItem::FleetTurn { id, notice, rows, handled } => self.fleet_turn(ix, id, notice, rows, *handled, page, look, cx),
            ThreadItem::Sketch(card) => self.sketch(ix, card, page, look, now, cx),
            ThreadItem::File { id, name, .. } => self.file_row(id, name, look),
            ThreadItem::Document(card) => self.document(ix, card, page, look, now, cx),
            ThreadItem::Plan(plan) => self.plan_card(ix, plan, page, look, now, cx),
            ThreadItem::Judged { id, at, artifact, .. } => self.judged_row(id, artifact, at, page, look, now, cx),
            ThreadItem::Change { id, at, what, from, to } => self.change(id, at, *what, from, to, look, now, cx),
            ThreadItem::NotStarted { id, at, cause, log } => self.not_started(ix, id, at, cause, log, page, look, now, cx),
        })
    }

    /// What stands after the items (A1.4).
    fn extra(&self, head: &ipc::ThreadHead, items: &[ThreadItem], page: &Page, at: usize, look: &Look, now: DateTime<Local>, cx: &Context<Self>) -> Option<AnyElement> {
        Some(match page.extras.get(at)? {
            Extra::Plan(_) => self.plan_card(page.items + at, head.plan.as_ref()?, page, look, now, cx),
            Extra::Sketch(_) => self.sketch_pin(page.items + at, head.sketch.as_ref()?, page, look, cx),
            Extra::Pending(i) => {
                let one = page.pending.get(*i)?;
                let delivery = if one.failed { Delivery::NotSent(one.key) } else { Delivery::Sending };
                self.you(page.items + at, page, &format!("pending:{}", one.key), &one.text, &[], &[], &one.tags, None, Some(delivery), false, false, None, look, now, cx)
            }
            Extra::Working => {
                let ink = look.color(token::STATUS_RUNNING);
                let line = labelled(
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
                        .child("Thinking")
                        .child(self.stop_button(look, cx)),
                    Role::Status,
                    "Working",
                );
                line.into_any_element()
            }
            Extra::Subagents => return self.subagents_card(page.items + at, items, page, look, cx),
            Extra::Waiting => return self.waiting_card(&self.shown.clone()?, look, cx),
            Extra::End => return self.end_card(&self.shown.clone()?, look, cx),
            Extra::MovedTo => {
                let id = self.shown.clone()?;
                let moved = self.moved(&id, "forked_to", cx)?;
                return Some(self.moved_row(&id, "to", moved, look, cx));
            }
            Extra::Refreshing => div()
                .flex()
                .py(look.length(token::SPACE_1))
                .child(labelled(
                    div().id("refreshing").flex_none().tooltip(tip("Loading new messages")).child(
                        glyph(look, "loader-circle", Size::Small, look.color(token::FG_MUTED)).with_animation(
                            "refreshing-turn",
                            Animation::new(look.duration(token::DURATION_PULSE)).repeat(),
                            |mark, delta| mark.with_transformation(gpui::Transformation::rotate(gpui::percentage(delta))),
                        ),
                    ),
                    Role::Image,
                    "Loading new messages",
                ))
                .into_any_element(),
        })
    }

    /// Where a moved Job went, at the foot of its thread, or where it came from, at the head of the new
    /// one (`MovedRow`): a press opens the other.
    fn moved_row(&self, id: &SharedString, way: &'static str, (other, manifest): (String, String), look: &Look, cx: &Context<Self>) -> AnyElement {
        let repository = self.repository_word(&manifest, cx);
        let said = format!("Moved {way} {repository}");
        let press = div()
            .id(SharedString::from(format!("moved-{way}:{id}")))
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .rounded(look.length(token::RADIUS_MD))
            .bg(look.color(token::BG_RAISED))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_MUTED))
            .cursor_pointer()
            .hover(|style| style.text_color(look.color(token::FG_DEFAULT)))
            .child(glyph(look, "folder-git-2", Size::Small, look.color(token::FG_MUTED)))
            .child(format!("Moved {way}"))
            .child(div().min_w_0().truncate().font_family(look.font_stack(token::FONT_MONO).into_iter().next().unwrap_or_default()).child(repository))
            .child(glyph(look, "chevron-right", Size::Small, look.color(token::FG_MUTED)))
            .on_click(cx.listener(move |this, _, _, cx| this.open_job(other.clone(), cx)));
        div().w_full().flex().child(labelled(press, Role::Button, said)).into_any_element()
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
        ix: usize,
        page: &Page,
        id: &str,
        text: &str,
        files: &[ipc::SentFile],
        sketches: &[ipc::SentSketch],
        tags: &[SessionTag],
        at: Option<&ipc::Instant>,
        delivery: Option<Delivery>,
        queued: bool,
        editable: bool,
        edited: Option<&ipc::Instant>,
        look: &Look,
        now: DateTime<Local>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let (said, debug) = split_debug(text);
        let debug = debug.map(|facts| {
            let key = format!("debug:{id}");
            let open = page.opened.contains(&key);
            let head = vec![
                glyph(look, if open { "chevron-down" } else { "chevron-right" }, Size::Small, look.color(token::FG_MUTED)).into_any_element(),
                div().child("Debug info").into_any_element(),
            ];
            self.fold(ix, key.clone(), "Debug info".into(), head, Some(self.prose(key, facts, look, cx)), page, look, cx)
        });
        let band = div()
            .flex()
            .items_center()
            .justify_end()
            .gap(look.length(token::SPACE_2))
            .px(look.length(token::SPACE_2))
            .text_color(look.color(token::FG_SUBTLE))
            .when(delivery == Some(Delivery::Sending), |band| {
                band.child(labelled(
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
                    Role::Image,
                    "Sending",
                ))
            })
            // Fleet did not take it: a cross, as `Delivery` draws one.
            .when(matches!(delivery, Some(Delivery::NotSent(_))), |band| {
                band.child(labelled(
                    div()
                        .id(SharedString::from(format!("not-sent:{id}")))
                        .flex()
                        .child(glyph(look, "x", Size::Small, look.color(token::FG_SUBTLE)))
                        .tooltip(tip("Not sent")),
                    Role::Image,
                    "Not sent",
                ))
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
            .when_some(edited, |band, edited| {
                let said = clock(edited.as_str(), now).map_or_else(|| "Edited".to_string(), |when| format!("Edited {when}"));
                band.child(labelled(
                    div()
                        .id(SharedString::from(format!("edited:{id}")))
                        .text_size(look.length(token::TEXT_2XS))
                        .line_height(look.length(token::LEADING_2XS))
                        .text_color(look.color(token::FG_MUTED))
                        .child("edited")
                        .tooltip(tip(said)),
                    Role::Group,
                    "Edited",
                ))
            })
            .children(at.map(|at| self.stamp(at, look, now, false, cx)));
        let (said_parts, loose) = place_tags(said, tags);
        // What the message was sent with, under its words inside the bubble (`Sent`): each picture drawn, the
        // other files, the sketches and the tags the words did not place as chips, each after its kind.
        let sent = (!files.is_empty() || !sketches.is_empty() || !loose.is_empty()).then(|| {
            div()
                .flex()
                .flex_wrap()
                .items_start()
                .gap(look.length(token::SPACE_2))
                .children(files.iter().map(|file| {
                    self.picture_tile(file, look, cx).unwrap_or_else(|| self.chip(None, &file.name, look).into_any_element())
                }))
                .children(sketches.iter().map(|one| {
                    let said = format!("Sketch · {}", one.title);
                    labelled(self.chip(Some("Sketch"), &one.title, look).id(SharedString::from(format!("sketch:{id}:{}", one.id))), Role::Group, said).into_any_element()
                }))
                .children(loose.iter().map(|tag| {
                    let said = format!("{} · {}", tag_kind(tag.kind), tag.title);
                    labelled(self.chip(Some(tag_kind(tag.kind)), &tag.title, look).id(SharedString::from(format!("loose:{id}:{}", tag.id))), Role::Group, said).into_any_element()
                }))
        });
        // Where Fleet did not take it, Retry sends it again; gone while it goes.
        let retry = match delivery {
            Some(Delivery::NotSent(key)) => self.shown.clone().map(|job| {
                div().flex().child(labelled(
                    super::acts::button(look, SharedString::from(format!("retry:{id}")), "Retry", super::acts::Tone::Secondary, false, None, true)
                        .h(look.length(token::H_CONTROL_SM))
                        .px(look.length(token::SPACE_2))
                        .text_size(look.length(token::TEXT_XS))
                        .line_height(look.length(token::LEADING_XS))
                        .on_click(cx.listener(move |this, _, window, cx| this.retry(&job, key, window, cx))),
                    Role::Button,
                    "Retry",
                ))
            }),
            _ => None,
        };
        let editing = editable.then(|| self.edit_box(id, look, cx)).flatten();
        let open = editing.is_some();
        let words = match editing {
            Some(field) => field,
            None => match said_parts {
                // Where his words place a tag, the tag is a chip in its line (`armada-session-said`).
                Some(parts) => {
                    let words = self.prose_tagged(format!("you:{id}"), &tagged_words(&parts, tags), token::FG_DEFAULT, tags, look, cx);
                    self.block(page, id, labelled(div().id(SharedString::from(format!("said:{id}"))).child(words), Role::Group, "Message said").into_any_element(), cx)
                }
                None => self.block(page, id, self.prose(format!("you:{id}"), said, look, cx), cx),
            },
        };
        let region = div()
            .id(SharedString::from(format!("you:{id}")))
            .flex()
            .flex_col()
            .items_end()
            .when(delivery == Some(Delivery::Sending), |row| row.opacity(0.6));
        let region = if editable {
            let spot = self.spot(page, &format!("you:{id}"), cx);
            let region = region.track_focus(&spot.focus);
            self.menu_on(region, &spot, "Your message", spot.focus.clone(), Rc::new(super::edit::entries), self.edit_pick(ix, id, text), cx)
        } else {
            region
        };
        labelled(
            region
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_end()
                        .gap(look.length(token::SPACE_2))
                        .min_w_0()
                        .max_w(relative(0.8))
                        // Open for new words, the bubble takes the room it may, so the box is not a sliver.
                        .when(open, |column| column.w(relative(0.8)).items_stretch())
                        .child(labelled(
                            div()
                                .id(SharedString::from(format!("words:{id}")))
                                // Held to the column, so a long line or list item wraps inside it
                                // rather than widening the bubble past the thread's edge.
                                .min_w_0()
                                .max_w_full()
                                .px(look.length(token::SPACE_4) + look.length(token::SPACE_1))
                                .py(look.length(token::SPACE_3))
                                .rounded(look.length(token::RADIUS_LG))
                                .bg(look.color(token::FG_DEFAULT).opacity(0.07))
                                .flex()
                                .flex_col()
                                .gap(look.length(token::SPACE_2))
                                .when(open, |words| words.border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::FOCUS_RING)))
                                .child(words)
                                .children(debug)
                                .children(sent)
                                .children(retry),
                            Role::Group,
                            "Message words",
                        ))
                        .child(band),
                ),
            Role::Region,
            "Your message",
        )
        .into_any_element()
    }

    /// A name drawn as a read-only chip: a file, or a tag the words did not place after its kind.
    fn chip(&self, from: Option<&str>, said: &str, look: &Look) -> gpui::Div {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_1))
            .max_w_full()
            .overflow_hidden()
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_SM))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .bg(look.color(token::BG_SUNKEN))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .text_color(look.color(token::FG_DEFAULT))
            .children(from.map(|from| div().flex_none().text_color(look.color(token::FG_SUBTLE)).child(format!("{from} ·"))))
            .child(div().min_w_0().truncate().child(said.to_string()))
    }

    /// The agent's words, with the bar under them that shows on hover (A2.3).
    #[allow(clippy::too_many_arguments)]
    fn agent(&self, id: &str, text: &str, at: &ipc::Instant, page: &Page, look: &Look, now: DateTime<Local>, window: &Window, cx: &Context<Self>) -> AnyElement {
        let group = SharedString::from(format!("agent:{id}"));
        div()
            .group(group)
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_2))
            .min_w_0()
            .child(labelled(div().id(SharedString::from(format!("agent-words:{id}"))).child(self.block(page, id, self.prose(format!("agent:{id}"), text, look, cx), cx)), Role::Group, "Agent message"))
            .child(self.message_bar(id, text, at, page, look, now, window, cx))
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
    pub(super) fn message(&self, ix: usize, row: &SessionRow, page: &Page, look: &Look, now: DateTime<Local>, window: &Window, cx: &Context<Self>) -> Option<AnyElement> {
        Some(match row {
            SessionRow::Message { id, at, from: SessionVoice::You, text, files, sketches, tags, queued, edited_at, .. } => {
                if let Some(act) = act_of(text) {
                    return Some(self.did(id, act, at, look, now, cx));
                }
                let text = page.edits.get(id).unwrap_or(text);
                self.you(ix, page, id, text, files, sketches, tags, Some(at), None, *queued, true, edited_at.as_ref(), look, now, cx)
            }
            SessionRow::Message { id, at, from: SessionVoice::Agent, text, .. } => self.agent(id, text, at, page, look, now, window, cx),
            SessionRow::Message { id, at, from: SessionVoice::Session { id: sender, title, .. }, text, notice, .. } => {
                // At a narrow width the id gives way faster than the title, so the title stays. Squeezed to
                // fewer than a few characters it leaves the band, and comes back once the band has that much
                // more room than it had then. Both widths are the last frame's.
                let band_spot = self.spot(page, &format!("from:{id}"), cx);
                let id_spot = self.spot(page, &format!("from-id:{id}"), cx);
                let few = {
                    let start: String = sender.chars().take(4).chain(std::iter::once('…')).collect();
                    let font = gpui::font(mono(look));
                    window.text_system().shape_line(start.clone().into(), look.length(token::TEXT_2XS), &[gpui::TextRun { len: start.len(), font, ..Default::default() }], None).width
                };
                let (band_wide, id_wide) = (band_spot.bounds.get().size.width, id_spot.bounds.get().size.width);
                let cramped_key = format!("from-id:{id}");
                let mut cramped = page.keys.cramped.borrow_mut();
                let shows_id = match cramped.get(&cramped_key).copied() {
                    Some(then) if band_wide <= then + few => false,
                    Some(_) => {
                        cramped.remove(&cramped_key);
                        true
                    }
                    None if band_wide > gpui::px(0.) && id_wide < few => {
                        cramped.insert(cramped_key, band_wide);
                        false
                    }
                    None => true,
                };
                drop(cramped);
                let measure = |cell: Rc<Cell<gpui::Bounds<gpui::Pixels>>>| {
                    canvas(
                        move |bounds, window, _| {
                            if cell.get().size.width != bounds.size.width {
                                cell.set(bounds);
                                window.refresh();
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                };
                let band = div()
                    .relative()
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
                    .child(measure(band_spot.bounds.clone()))
                    .child(glyph(look, "square-terminal", Size::Small, look.color(token::FG_MUTED)))
                    .when(shows_id, |band| {
                        band.child(
                            div()
                                .relative()
                                .min_w_0()
                                .flex_shrink(4.)
                                .truncate()
                                .font_family(mono(look))
                                .text_size(look.length(token::TEXT_2XS))
                                .text_color(look.color(token::FG_MUTED))
                                .child(sender.clone())
                                .child(measure(id_spot.bounds.clone())),
                        )
                    })
                    .child(
                        div()
                            .flex_auto()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight(look.number(token::WEIGHT_MEDIUM)))
                            .child(title.clone()),
                    )
                    .child(self.stamp(at, look, now, true, cx));
                let band = if sender == "fleet" {
                    labelled(band.id(SharedString::from(format!("from:{id}"))).track_focus(&band_spot.focus), Role::Button, format!("Fleet notice {title}")).into_any_element()
                } else {
                    self.opens(sender, format!("from:{id}"), format!("Open Job {title}"), cx, band)
                };
                let card_spot = self.spot(page, &format!("from-card:{id}"), cx);
                let said = page.cards.said.get(&format!("notice:{id}")).cloned();
                let card = div()
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
                            .child(self.block(page, id, self.prose(format!("from:{id}"), text, look, cx), cx)),
                    )
                    .children(said.map(|why| refusal(look, &format!("notice:{id}"), why)));
                let card = if sender == "fleet" {
                    self.menu_on(
                        card,
                        &card_spot,
                        "Fleet notice",
                        band_spot.focus.clone(),
                        Rc::new({
                            let word = notice.clone();
                            move || notice_entries(word.as_deref())
                        }),
                        self.notice_pick(&format!("notice:{id}"), text.clone(), Some(at.clone()), notice.clone()),
                        cx,
                    )
                } else {
                    card
                };
                labelled(card, Role::Region, format!("Message from {title}")).into_any_element()
            }
            _ => return None,
        })
    }
}

/// The words before a line that reads `## Debug info`, and the facts after it.
fn split_debug(text: &str) -> (&str, Option<&str>) {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "## Debug info" {
            let facts = text[at + line.len()..].trim();
            return (text[..at].trim_end(), Some(facts).filter(|facts| !facts.is_empty()));
        }
        at += line.len();
    }
    (text, None)
}

/// How long a block's Copy reads "Copied" after a press, as the message bar's.
const CODE_COPIED_FOR: std::time::Duration = std::time::Duration::from_millis(1500);

/// Copy over a fenced block's corner, as Electron's `CopyAct`: shown while the pointer is on the
/// words, a check and "Copied" for a moment after a press. The fence's closing newline is not copied.
fn code_copy(look: &Look, job: String, key: String, base: usize) -> impl Fn(&gpui_base::text::CodeBlock, &mut Window, &mut gpui::App) -> AnyElement + Send + Sync + 'static {
    let (size, glyph_size, radius) = (look.length(token::H_CONTROL_SM), look.length(token::SPACE_4), look.length(token::RADIUS_MD));
    let (ink, hover) = (look.color(token::FG_SUBTLE), look.color(token::BG_HOVER));
    move |block, window, cx| {
        let at = block.span.map(|span| span.start).unwrap_or_default();
        let copied = window.use_keyed_state(ElementId::Name(format!("code-copied:{at}").into()), cx, |_, _| false);
        let shown = *copied.read(cx);
        let name: SharedString = if shown { "Copied" } else { "Copy" }.into();
        let code = block.code().trim_end_matches('\n').to_string();
        let copy = labelled(
            div()
                .id("copy")
                .flex()
                .items_center()
                .justify_center()
                .size(size)
                .rounded(radius)
                .cursor_pointer()
                .opacity(if shown { 1. } else { 0. })
                .group_hover(PROSE_GROUP, |style| style.opacity(1.))
                .hover(|style| style.bg(hover))
                .tooltip(tip(name.clone()))
                .child(svg().path(icon_path(if shown { "check" } else { "copy" })).size(glyph_size).text_color(ink))
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(code.clone()));
                    copied.update(cx, |copied, cx| {
                        *copied = true;
                        cx.notify();
                    });
                    let copied = copied.clone();
                    cx.spawn(async move |cx| {
                        cx.background_executor().timer(CODE_COPIED_FOR).await;
                        copied.update(cx, |copied, cx| {
                            *copied = false;
                            cx.notify();
                        });
                    })
                    .detach();
                }),
            Role::Button,
            name,
        )
        .into_any_element();
        // Run beside Copy on a shell block, as Electron's `RunnableBlock` (`terminal::run`).
        let (start, end) = block.span.map_or((0, 0), |span| (base + span.start, base + span.end));
        let command = crate::terminal::run::command_of(block.lang().as_deref(), &block.code());
        let look = cx.look().clone();
        crate::terminal::run::block_acts(&job, format!("{key}{}{start}", crate::terminal::run::AT), end, command, copy, PROSE_GROUP, &look, cx)
    }
}

/// The group a message's words hover as, so a block's Copy shows while the pointer is on them.
const PROSE_GROUP: &str = "prose";

/// Where one of his messages stands while Fleet has not taken it (`Delivery`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Delivery {
    Sending,
    /// Fleet did not take it; the pending message's key, for Retry.
    NotSent(u64),
}

/// One stretch of his words: as written, or a tag at the place it was written.
#[derive(Debug, PartialEq)]
pub(super) enum Said<'a> {
    Words(&'a str),
    Tag(&'a SessionTag),
}

/// His words with each tag at its `@title`, found in order after the one before (`placeTags`): `None`
/// where no tag is found, and the tags that were not.
pub(super) fn place_tags<'a>(text: &'a str, tags: &'a [SessionTag]) -> (Option<Vec<Said<'a>>>, Vec<&'a SessionTag>) {
    let mut parts = Vec::new();
    let mut loose = Vec::new();
    let mut from = 0;
    for tag in tags {
        let needle = format!("@{}", tag.title);
        match text[from..].find(&needle) {
            None => loose.push(tag),
            Some(found) => {
                let at = from + found;
                if at > from {
                    parts.push(Said::Words(&text[from..at]));
                }
                parts.push(Said::Tag(tag));
                from = at + needle.len();
            }
        }
    }
    if parts.is_empty() {
        return (None, loose);
    }
    if from < text.len() {
        parts.push(Said::Words(&text[from..]));
    }
    (Some(parts), loose)
}

/// What a tag's kind is called (`TAG_KIND`).
pub(super) fn tag_kind(kind: TagKind) -> &'static str {
    match kind {
        TagKind::Session => "Job",
        TagKind::PullRequest => "Pull request",
        TagKind::Branch => "Branch",
        TagKind::Issue => "Issue",
        TagKind::Report => "Reported bug",
        TagKind::Terminal => "Terminal",
    }
}

/// What marks a tag in the markdown his words are drawn from: inline code that reads this and the tag's place.
const TAG_MARK: char = '\u{E000}';

/// His words with each placed tag written as its mark, a Job's inside a link to it, so the markdown carries it.
fn tagged_words(parts: &[Said], tags: &[SessionTag]) -> String {
    let mut words = String::new();
    for part in parts {
        match part {
            Said::Words(text) => words.push_str(text),
            Said::Tag(tag) => {
                let at = tags.iter().position(|one| std::ptr::eq(one, *tag)).unwrap_or_default();
                match tag.kind {
                    TagKind::Session => words.push_str(&format!("[`{TAG_MARK}{at}`](armada-ref:session/{})", tag.id)),
                    _ => words.push_str(&format!("`{TAG_MARK}{at}`")),
                }
            }
        }
    }
    words
}

/// The markdown extension that draws each tag's mark as its chip.
fn tag_extensions(tags: &[SessionTag], look: &Look) -> MarkdownExtensions {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for tag in tags {
        std::hash::Hash::hash(&(&tag.id, &tag.title, tag_kind(tag.kind)), &mut hash);
    }
    let revision = std::hash::Hasher::finish(&hash) | 1;
    MarkdownExtensions::default().plugin(TagPlugin { tags: tags.to_vec(), paint: TagPaint::of(look) }).parser_revision(revision)
}

/// The look an `InlineTag` chip is drawn with, read once so the plugin holds no `Look`.
#[derive(Clone, Copy)]
struct TagPaint {
    accent: Hsla,
    ink: Hsla,
    glyph: Hsla,
    border: Hsla,
    fill: Hsla,
    gap: gpui::Pixels,
    pad: gpui::Pixels,
    radius: gpui::Pixels,
    rule: gpui::Pixels,
    size: gpui::Pixels,
    leading: gpui::Pixels,
    icon: gpui::Pixels,
}

impl TagPaint {
    fn of(look: &Look) -> Self {
        TagPaint {
            accent: look.color(token::ACCENT),
            ink: look.color(token::FG_DEFAULT),
            glyph: look.color(token::FG_MUTED),
            border: look.color(token::BORDER_DEFAULT),
            fill: look.color(token::BG_RAISED),
            gap: look.length(token::SPACE_1),
            pad: look.length(token::SPACE_1),
            radius: look.length(token::RADIUS_MD),
            rule: look.length(token::BORDER_WIDTH),
            size: look.length(token::TEXT_XS),
            leading: look.length(token::LEADING_XS),
            icon: Size::Small.of(look),
        }
    }
}

/// A tag in his words as `InlineTag` draws it: its kind's glyph and title in one unit. A Job's is accent, a link
/// the markdown carries, with its kind and title on hover.
struct TagPlugin {
    tags: Vec<SessionTag>,
    paint: TagPaint,
}

impl MarkdownPlugin for TagPlugin {
    fn name(&self) -> &str {
        "armada-tag"
    }

    fn parse(&self, node: &markdown_ast::Node, _: &MarkdownParseContext<'_>) -> Option<MarkdownNode> {
        let markdown_ast::Node::InlineCode(code) = node else { return None };
        let tag = self.tags.get(code.value.strip_prefix(TAG_MARK)?.parse::<usize>().ok()?)?;
        let said = format!("@{}", tag.title);
        Some(MarkdownNode::new("armada-tag", tag.clone()).text(said.clone()).markdown(said).accessibility_label(tag.title.clone()))
    }

    fn render_inline(&self, node: &MarkdownNode, _: &InlineRenderContext, _: &mut Window, _: &mut App) -> Option<InlineElement> {
        let tag = node.data::<SessionTag>()?;
        let paint = self.paint;
        let link = tag.kind == TagKind::Session;
        let ink = if link { paint.accent } else { paint.ink };
        let chip = div()
            .id(SharedString::from(format!("tag:{}:{}", tag_kind(tag.kind), tag.id)))
            .flex()
            .items_center()
            .gap(paint.gap)
            .px(paint.pad)
            .rounded(paint.radius)
            .border(paint.rule)
            .border_color(paint.border)
            .bg(paint.fill)
            .text_size(paint.size)
            .line_height(paint.leading)
            .text_color(ink)
            .text_decoration_0()
            .text_decoration_color(gpui::transparent_black())
            .whitespace_nowrap()
            .child(svg().path(icon_path(tag_glyph(tag.kind))).flex_none().size(paint.icon).text_color(if link { ink } else { paint.glyph }))
            .child(tag.title.clone());
        let chip = if link {
            labelled(chip.cursor_pointer().tooltip(tip(format!("{} · {}", tag_kind(tag.kind), tag.title))), Role::Link, tag.title.clone()).into_any_element()
        } else {
            labelled(chip, Role::Group, tag.title.clone()).into_any_element()
        };
        Some(InlineElement::new(chip))
    }
}

/// A tag kind's glyph (`TAG_GLYPH`).
fn tag_glyph(kind: TagKind) -> &'static str {
    match kind {
        TagKind::Session => "square-terminal",
        TagKind::PullRequest => "git-pull-request",
        TagKind::Branch => "git-branch",
        TagKind::Issue => "ticket",
        TagKind::Report => "bug",
        TagKind::Terminal => "terminal",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lone_line_break_is_kept_and_code_is_left_as_written() {
        assert_eq!(with_breaks("one\ntwo"), "one  \ntwo");
        assert_eq!(with_breaks("para\n\nnext"), "para\n\nnext");
        assert_eq!(with_breaks("```\na\nb\n```\nafter"), "```\na\nb\n```\nafter");
        assert_eq!(with_breaks("say\n```\ncode\n```"), "say\n```\ncode\n```");
    }

    fn tag(kind: TagKind, id: &str, title: &str) -> SessionTag {
        SessionTag { kind, id: id.into(), title: title.into(), url: None }
    }

    #[test]
    fn a_tag_is_placed_at_its_title_in_order_and_the_rest_are_loose() {
        let tags = [tag(TagKind::Session, "s1", "Native Bridge foundation"), tag(TagKind::PullRequest, "3290", "#3290 Merge line"), tag(TagKind::Session, "s2", "Audit")];
        let (parts, loose) = place_tags("Ask @Native Bridge foundation, then @Audit.", &tags);
        assert_eq!(
            parts,
            Some(vec![Said::Words("Ask "), Said::Tag(&tags[0]), Said::Words(", then "), Said::Tag(&tags[2]), Said::Words(".")])
        );
        assert_eq!(loose, vec![&tags[1]]);
        let (parts, loose) = place_tags("No tag written", &tags[..1]);
        assert_eq!((parts, loose), (None, vec![&tags[0]]));
    }

    #[test]
    fn debug_info_is_split_off_at_its_heading() {
        assert_eq!(split_debug("Fix it\n\n## Debug info\nbuild 12\nmain"), ("Fix it", Some("build 12\nmain")));
        assert_eq!(split_debug("No facts here"), ("No facts here", None));
        assert_eq!(split_debug("Words\n## Debug info"), ("Words", None));
    }

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

