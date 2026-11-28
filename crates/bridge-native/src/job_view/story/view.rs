//! The Story tab drawn from what `model.rs` made of Fleet's read: filter tabs over a timeline of rows, each one line with its
//! glyph, its title and what it says, then the portion of time it took and when it was. An opened row is itself the card:
//! bordered in the accent, with the chat of it and what it links to under its line. Every value is a token.

use gpui::{div, list, prelude::*, px, relative, AnyElement, Div, FontWeight, Hsla, IntoElement, Render, Role, SharedString, Window};
use gpui_component::text::TextView;

use super::model::{pull_events, Bubble, Entry, Event, Filter, Link, Stand, Who};
use super::{Load, Region, StoryTab};
use crate::features::asks::hue_token;
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::settings::view::{bone, ring_colour, tip};
use crate::titlebar::parts::{glyph, mono, sans};

/// The edge of the opened row: the accent at 45% over the default edge.
fn opened_edge(look: &Look) -> Hsla {
    look.color(token::BORDER_DEFAULT).blend(look.color(token::ACCENT).opacity(0.45))
}

fn muted_line(look: &Look, words: &str) -> Div {
    div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(words.to_string()))
}

/// The three columns every row ends in, each one width, so the figures line up down the list.
fn column(look: &Look, width: gpui::Pixels) -> Div {
    div().flex_none().w(width).flex().items_center().justify_end().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).whitespace_nowrap()
}

/// One filter tab, picked or not; `on` rings it where the keys are on it. The Artifacts tab draws its filters with it too.
pub(crate) fn filter_tab(look: &Look, id: SharedString, word: &'static str, picked: bool, on: bool) -> gpui::Stateful<Div> {
    let edge = look.length(token::BORDER_WIDTH);
    div()
        .id(id)
        .flex_none()
        .flex()
        .items_center()
        .h(look.length(token::H_CONTROL) - look.length(token::SPACE_1) - edge * 2.)
        .px(look.length(token::SPACE_3))
        .rounded(look.length(token::RADIUS_MD))
        .border(edge)
        .border_color(if on { ring_colour(look) } else { gpui::transparent_black() })
        .text_size(look.length(token::TEXT_XS))
        .line_height(look.length(token::LEADING_XS))
        .whitespace_nowrap()
        .cursor_pointer()
        .map(|tab| {
            if picked {
                tab.bg(look.color(token::ACCENT_MUTED)).border_color(if on { ring_colour(look) } else { look.color(token::ACCENT) }).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT))
            } else {
                let hover = look.color(token::FG_DEFAULT);
                tab.text_color(look.color(token::FG_MUTED)).hover(move |style| style.text_color(hover))
            }
        })
        .child(word)
}

/// The row of filter tabs over a list, its edge `inset` in from the pane's, named `name` for VoiceOver.
pub(crate) fn filter_row(look: &Look, id: &'static str, name: &'static str, inset: gpui::Pixels, tabs: Vec<AnyElement>) -> AnyElement {
    let track = crate::parts::tabs::track(look, id).children(tabs);
    div().px(inset).pt(look.length(token::SPACE_4)).pb(look.length(token::SPACE_2)).flex().child(labelled(track, Role::TabList, name)).into_any_element()
}

impl StoryTab {
    /// One filter tab. The keys' own place on it is a ring.
    fn render_filter(&self, filter: Filter, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let picked = self.filter == filter;
        let on = picked && self.region == Region::Filters && self.focus.is_focused(window);
        let tab = filter_tab(look, SharedString::from(format!("story-filter-{}", filter.word())), filter.word(), picked, on).on_click(cx.listener(move |this, _, window, cx| {
            window.focus(&this.focus, cx);
            this.region = Region::Filters;
            this.filter_by(filter, cx);
        }));
        labelled(tab, Role::Tab, filter.word()).into_any_element()
    }

    fn render_filters(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> Option<AnyElement> {
        let tabs = self.built.filters();
        if tabs.len() < 2 {
            return None;
        }
        let tabs = tabs.into_iter().map(|filter| self.render_filter(filter, look, window, cx)).collect();
        Some(filter_row(look, "story-filters", "Story filter", look.length(token::SPACE_6), tabs))
    }

    /// The fixed-width pill that says where an ask stands, with what it means in its tooltip.
    fn render_stand(&self, key: &str, stand: Stand, means: &'static str, look: &Look) -> AnyElement {
        let ink = look.color(hue_token(stand.hue));
        labelled(
            div()
                .id(SharedString::from(format!("story-stand-{key}")))
                .flex_none()
                .w(look.length(token::SPACE_12) + look.length(token::SPACE_12) / 2.)
                .h(look.length(token::H_BADGE))
                .flex()
                .items_center()
                .justify_center()
                .gap(look.length(token::SPACE_1))
                .rounded(look.length(token::RADIUS_MD))
                .bg(ink.opacity(0.12))
                .text_size(look.length(token::TEXT_2XS))
                .text_color(ink)
                .whitespace_nowrap()
                .tooltip(tip(means))
                .child(glyph(stand.glyph, look.length(token::SPACE_3), ink))
                .child(stand.word),
            Role::Status,
            format!("{}, {means}", stand.word),
        )
        .into_any_element()
    }

    /// One line of the chat: his on the right in the accent, the Job's on the left. His words are drawn as the thread draws
    /// them, so a brief's headings, lists and code read as formatted.
    fn render_bubble(&self, key: &str, at: usize, bubble: &Bubble, look: &Look) -> AnyElement {
        let (fill, edge) = if bubble.mine { (look.color(token::ACCENT_MUTED), look.color(token::ACCENT)) } else { (look.color(token::BG_OVERLAY), look.color(token::BORDER_DEFAULT)) };
        let under = match (bubble.mine, bubble.when.is_empty()) {
            (true, true) => String::new(),
            (true, false) => bubble.when.clone(),
            (false, true) => bubble.who.clone(),
            (false, false) => format!("{} · {}", bubble.who, bubble.when),
        };
        let id = format!("story-bubble-{key}-{at}");
        let body = div()
            .max_w(relative(0.8))
            .bg(fill)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .rounded(look.length(token::RADIUS_LG))
            .px(look.length(token::SPACE_3))
            .py(look.length(token::SPACE_2))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .child(TextView::markdown(gpui::ElementId::Name(id.clone().into()), bubble.said.clone()).text_color(look.color(token::FG_DEFAULT)));
        let line = div()
            .id(SharedString::from(id))
            .w_full()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_1))
            .map(|line| if bubble.mine { line.items_end() } else { line.items_start() })
            .child(body)
            .when(!under.is_empty(), |line| line.child(muted_line(look, &under)));
        let who = if bubble.mine { "You".to_string() } else { bubble.who.clone() };
        labelled(line, Role::Group, format!("{who}: {}", bubble.said)).into_any_element()
    }

    /// A line of the opened card that goes somewhere: the keys' place on it is a wash.
    fn press_line(&self, id: String, name: String, link: Link, picked: bool, look: &Look, cx: &mut gpui::Context<Self>) -> gpui::Stateful<Div> {
        let _ = name;
        div()
            .id(SharedString::from(id))
            .w_full()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_3))
            .px(look.length(token::SPACE_2))
            .py(look.length(token::SPACE_1))
            .rounded(look.length(token::RADIUS_MD))
            .text_size(look.length(token::TEXT_XS))
            .line_height(look.length(token::LEADING_XS))
            .cursor_pointer()
            .when(picked, |line| line.bg(look.color(token::ACCENT).opacity(0.12)))
            .when(!picked, |line| line.hover(|style| style.bg(look.color(token::BG_HOVER))))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.region = Region::Links;
                window.focus(&this.focus, cx);
                this.go(link.clone(), window, cx);
            }))
    }

    fn render_event(&self, key: &str, at: usize, event: &Event, picked: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let id = format!("story-event-{key}-{at}");
        let mark = glyph(event.glyph, look.length(token::SPACE_4), look.color(hue_token(event.hue)));
        let sha = div().flex_none().w(look.length(token::SPACE_12) + look.length(token::SPACE_3)).font_family(mono(look)).text_color(look.color(token::FG_SUBTLE)).children(event.sha.clone().map(SharedString::from));
        let said = div().flex_1().min_w_0().truncate().text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(event.said.clone()));
        let when = column(look, look.length(token::SPACE_12) + look.length(token::SPACE_8)).child(SharedString::from(event.when.clone()));
        match event.link.clone() {
            Some(link) => labelled(
                self.press_line(id, event.said.clone(), link, picked, look, cx).tooltip(tip("Shows the files it changed")).child(mark).child(said).child(sha).child(when),
                Role::Button,
                event.said.clone(),
            )
            .into_any_element(),
            None => div().id(SharedString::from(id)).w_full().flex().items_center().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_2)).py(look.length(token::SPACE_1)).text_size(look.length(token::TEXT_XS)).child(mark).child(said).child(sha).child(when).into_any_element(),
        }
    }

    /// What an opened row holds under its line.
    fn render_card(&self, flat: usize, entry: &Entry, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let in_links = self.focus.is_focused(window) && self.region == Region::Links && self.cursor == Some(flat);
        let mut link_at = 0;
        let mut next = |on: bool| {
            let mine = on && in_links && self.link_at == link_at;
            link_at += 1;
            mine
        };
        let mut card = div().id(SharedString::from(format!("story-card-{}", entry.key))).w_full().flex().flex_col().gap(look.length(token::SPACE_3)).px(look.length(token::SPACE_4)).pb(look.length(token::SPACE_4)).pt(look.length(token::SPACE_1));
        if let Some(stand) = entry.card.stand {
            card = card.child(div().flex().child(self.render_stand(&entry.key, stand, entry.card.means, look)));
        }
        if !entry.card.about.is_empty() {
            card = card.child(muted_line(look, &entry.card.about));
        }
        if !entry.card.bubbles.is_empty() {
            let bubbles: Vec<AnyElement> = entry.card.bubbles.iter().enumerate().map(|(at, bubble)| self.render_bubble(&entry.key, at, bubble, look)).collect();
            card = card.child(div().w_full().flex().flex_col().gap(look.length(token::SPACE_2)).children(bubbles));
        }
        let filed: Vec<AnyElement> = entry
            .card
            .filed
            .iter()
            .map(|filed| {
                let picked = next(true);
                let line = self.press_line(format!("story-filed-{}-{}", entry.key, filed.slug), filed.title.clone(), Link::Feature(filed.slug.clone()), picked, look, cx);
                labelled(
                    line.text_color(look.color(token::ACCENT)).child(glyph("library", look.length(token::SPACE_4), look.color(token::FG_SUBTLE))).child(SharedString::from(format!("Filed under {}", filed.title))),
                    Role::Link,
                    format!("Filed under {}", filed.title),
                )
                .into_any_element()
            })
            .collect();
        if !filed.is_empty() {
            card = card.child(div().w_full().flex().flex_col().children(filed));
        }
        if let Some(merge) = &entry.card.merge {
            let commits = self.commits.get(&merge.number).map(Vec::as_slice).unwrap_or_default();
            let events: Vec<AnyElement> = pull_events(merge, commits, chrono::Utc::now()).iter().enumerate().map(|(at, event)| self.render_event(&entry.key, at, event, next(event.link.is_some()), look, cx)).collect();
            card = card.child(div().w_full().flex().flex_col().children(events));
            let picked = next(true);
            card = card.child(self.render_open(format!("story-open-pull-{}", entry.key), "Open pull request", Link::Pull(merge.number), picked, look, cx));
        }
        if let Some((id, title)) = &entry.card.job {
            let picked = next(true);
            let said = if title.is_empty() { "Open the Job".to_string() } else { format!("Open {title}") };
            card = card.child(self.render_open(format!("story-open-job-{}", entry.key), &said, Link::Job(id.clone()), picked, look, cx));
        }
        card.into_any_element()
    }

    /// A button that goes where a press in the card goes.
    fn render_open(&self, id: String, said: &str, link: Link, picked: bool, look: &Look, cx: &mut gpui::Context<Self>) -> AnyElement {
        let edge = look.length(token::BORDER_WIDTH);
        let button = div()
            .id(SharedString::from(id))
            .flex_none()
            .self_start()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .h(look.length(token::H_CONTROL) - look.length(token::SPACE_2))
            .px(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(edge)
            .border_color(if picked { ring_colour(look) } else { look.color(token::ACCENT) })
            .bg(look.color(token::ACCENT_MUTED))
            .text_size(look.length(token::TEXT_XS))
            .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
            .text_color(look.color(token::FG_DEFAULT))
            .whitespace_nowrap()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.region = Region::Links;
                window.focus(&this.focus, cx);
                this.go(link.clone(), window, cx);
            }))
            .child(glyph("link", look.length(token::SPACE_4), look.color(token::FG_DEFAULT)))
            .child(SharedString::from(said.to_string()));
        labelled(button, Role::Button, said.to_string()).into_any_element()
    }

    /// One row: its glyph, its title with what it says under it, then the portion of time and when. The timing wraps under the title when the pane is too narrow to hold both.
    fn render_entry(&self, flat: usize, entry: &Entry, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let open = self.unfolded.as_deref() == Some(entry.key.as_str());
        let on = self.focus.is_focused(window) && self.cursor == Some(flat) && self.region == Region::Rows;
        let hue = look.color(hue_token(entry.kind.hue()));
        let mark = labelled(
            div().id(SharedString::from(format!("story-kind-{}", entry.key))).flex_none().flex().items_center().justify_center().size(look.length(token::SPACE_4)).tooltip(tip(entry.kind.said())).child(glyph(entry.kind.glyph(), look.length(token::SPACE_4), hue)),
            Role::Image,
            entry.kind.said(),
        );
        let words = div()
            .flex_1()
            .min_w(look.length(token::SPACE_12) * 4.)
            .flex()
            .flex_col()
            .gap(px(1.))
            .child(div().w_full().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(entry.title.clone())))
            .when(!entry.says.is_empty(), |words| words.child(div().w_full().truncate().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_SUBTLE)).child(SharedString::from(entry.says.clone()))));
        let fill = look.color(if entry.spent.who == Who::You { token::STATUS_AWAITING_REVIEW } else { token::ACCENT });
        let spent_text = column(look, look.length(token::SPACE_12)).child(SharedString::from(entry.spent.said()));
        let spent_bar = div()
            .flex_none()
            .w(look.length(token::SPACE_12))
            .h(look.length(token::SPACE_1))
            .rounded_full()
            .bg(look.color(token::BORDER_SUBTLE))
            .when(entry.spent.share > 0, |bar| bar.child(div().h_full().w(relative(f32::from(entry.spent.share) / 100.)).rounded_full().bg(fill)));
        let stamp = column(look, look.length(token::SPACE_12) + look.length(token::SPACE_4)).child(SharedString::from(entry.when.clone()));
        let spent_said = match entry.spent.who {
            Who::You => "Time on you",
            Who::Job => "Time on the Job",
        };
        let spent = labelled(
            div().id(SharedString::from(format!("story-spent-{}", entry.key))).flex_none().flex().items_center().gap(look.length(token::SPACE_2)).tooltip(tip(spent_said)).child(spent_text).child(spent_bar),
            Role::Image,
            if entry.spent.minutes.is_some() { format!("{spent_said}, {}", entry.spent.said()) } else { spent_said.to_string() },
        );
        let line = div()
            .id(SharedString::from(format!("story-row-{}", entry.key)))
            .w_full()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x(look.length(token::SPACE_3))
            .gap_y(look.length(token::SPACE_1))
            .px(look.length(token::SPACE_4))
            .py(look.length(token::SPACE_3))
            .rounded(look.length(token::RADIUS_MD))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(if on { ring_colour(look) } else { gpui::transparent_black() })
            .when(on, |line| line.bg(look.color(token::ACCENT).opacity(0.12)))
            .when(!on, |line| line.hover(|style| style.bg(look.color(token::BG_HOVER))))
            .text_size(look.length(token::TEXT_SM))
            .line_height(look.length(token::LEADING_SM))
            .cursor_pointer()
            .aria_expanded(open)
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.focus, cx);
                this.toggle(flat, cx);
            }))
            .child(mark)
            .child(words)
            .child(div().flex_none().flex().items_center().gap(look.length(token::SPACE_3)).child(spent).child(stamp));
        let named = if entry.says.is_empty() { format!("{}, {}", entry.title, entry.when) } else { format!("{}, {}, {}", entry.title, entry.says, entry.when) };
        let line = labelled(line, Role::Button, named.clone());
        let frame = div().id(SharedString::from(format!("story-entry-{}", entry.key))).w_full().flex().flex_col();
        let framed = if open {
            frame.bg(look.color(token::BG_RAISED)).border(look.length(token::BORDER_WIDTH)).border_color(opened_edge(look)).rounded(look.length(token::RADIUS_LG)).child(line).child(self.render_card(flat, entry, look, window, cx))
        } else {
            frame.border_b(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE)).child(line)
        };
        labelled(framed, Role::ListBoxOption, named).into_any_element()
    }

    /// The heading over what he was shown before any ask.
    fn render_shown_heading(&self, look: &Look) -> AnyElement {
        labelled(
            div()
                .id("story-before-heading")
                .w_full()
                .pt(look.length(token::SPACE_4))
                .pb(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .line_height(look.length(token::LEADING_XS))
                .font_weight(FontWeight(look.number(token::WEIGHT_HEADING)))
                .text_color(look.color(token::FG_MUTED))
                .child("Shown to you"),
            Role::Heading,
            "Shown to you",
        )
        .into_any_element()
    }

    fn render_item(&self, ix: usize, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        let timeline = self.timeline_len();
        let inner = if ix < timeline {
            let rows = self.rows();
            rows.get(ix).map(|entry| self.render_entry(ix, entry, look, window, cx))
        } else if ix == timeline {
            Some(self.render_shown_heading(look))
        } else {
            let rows = self.rows();
            rows.get(ix - 1).map(|entry| self.render_entry(ix - 1, entry, look, window, cx))
        };
        div().w_full().px(look.length(token::SPACE_6)).pt(if ix == 0 { look.length(token::SPACE_2) } else { px(0.) }).pb(look.length(token::SPACE_2)).children(inner).into_any_element()
    }

    /// Rows of bone shapes, while the story is read.
    fn render_bones(&self, look: &Look) -> AnyElement {
        let mut bones = div().w_full().flex().flex_col().gap(look.length(token::SPACE_4)).px(look.length(token::SPACE_6)).py(look.length(token::SPACE_4));
        for at in 0..3 {
            bones = bones.child(
                div()
                    .id(SharedString::from(format!("story-bone-{at}")))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(look.length(token::SPACE_3))
                    .child(bone(look, look.length(token::SPACE_4), look.length(token::SPACE_4)))
                    .child(bone(look, px(150. + 36. * (at % 3) as f32), look.length(token::LEADING_SM) - px(4.)))
                    .child(div().flex_1())
                    .child(bone(look, look.length(token::SPACE_12), look.length(token::LEADING_XS) - px(4.))),
            );
        }
        labelled(bones.id("story-loading"), Role::Image, "Loading").into_any_element()
    }

    fn render_body(&self, look: &Look, window: &Window, cx: &mut gpui::Context<Self>) -> AnyElement {
        // A Job on a Mac that cannot be reached is not read, and no error says so: the Mac's state does, quietly.
        if matches!(self.story, Load::Failed(_)) {
            let mac = self.fleet().and_then(|fleet| fleet.read(cx).held().jobs.iter().find(|one| one.id.as_str() == self.job).and_then(|one| one.mac.clone()));
            if let Some((said, tip)) = crate::pairing::unreachable_line_of(mac.as_deref(), cx) {
                return crate::pairing::quiet_line(look, said, tip);
            }
        }
        match &self.story {
            Load::Reading => self.render_bones(look),
            Load::Failed(why) => div()
                .px(look.length(token::SPACE_6))
                .py(look.length(token::SPACE_4))
                .flex()
                .flex_col()
                .gap(look.length(token::SPACE_1))
                .text_size(look.length(token::TEXT_SM))
                .text_color(look.color(token::FG_MUTED))
                .child(div().font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Not read"))
                .child(SharedString::from(why.clone()))
                .into_any_element(),
            Load::Read(_) if self.built.entries.is_empty() && self.built.before.is_empty() => div().into_any_element(),
            Load::Read(_) => div()
                .size_full()
                .flex()
                .flex_col()
                .children(self.render_filters(look, window, cx))
                .child(labelled(
                    div().id("story-asks").flex_1().min_h_0().child(list(self.list.clone(), cx.processor(move |this, ix: usize, window, cx| {
                        let look = cx.look().clone();
                        this.render_item(ix, &look, window, cx)
                    })).size_full()),
                    Role::ListBox,
                    "Story",
                ))
                .into_any_element(),
        }
    }
}

impl Render for StoryTab {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let look = cx.look().clone();
        let body = self.render_body(&look, window, cx);
        let overlay = self.overlay().cloned().map(|overlay| div().id("story-overlay").absolute().top_0().left_0().size_full().occlude().child(overlay));
        div()
            .id(if self.nested { "story-nested" } else { "story-tab" })
            .key_context(crate::generated::actions::ActionScope::Features.key_context().unwrap_or_default())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_action(cx.listener(Self::on_act))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(look.color(token::BG_BASE))
            .font_family(sans(&look))
            .text_color(look.color(token::FG_DEFAULT))
            .child(div().flex_1().min_h_0().child(body))
            .children(overlay)
    }
}
