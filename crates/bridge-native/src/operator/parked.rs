//! The ideas the Operator kept for later, listed on its page, newest first. A row says what he sent, the kind he
//! called it where he called it one, and how long ago; Start sends it as a dispatch, and Remove lets it go.
//!
//! This draws what Fleet keeps (`list_parked_ideas`) and asks the page for each press, which holds the wire. An empty
//! list draws nothing, and a list not read yet draws rows where the words will be.

use std::collections::HashSet;

use gpui::{div, prelude::*, AnyElement, Context, Div, EventEmitter, FontWeight, Role, SharedString, Window};
use ipc::ParkedIdea;

use super::composer::pressed;
use super::face::dispatch_kind;
use super::view::{chip, press, Weight};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look};
use crate::titlebar::parts::{glyph, sans, tip};

/// What the list asks of the page.
#[derive(Clone, Debug, PartialEq)]
pub enum ParkedAsked {
    /// Send the idea as a dispatch, then let it go.
    Start(ParkedIdea),
    Remove(String),
}

impl EventEmitter<ParkedAsked> for ParkedIdeas {}

pub struct ParkedIdeas {
    /// What Fleet keeps, oldest first as it says it. `None` until read.
    ideas: Option<Vec<ParkedIdea>>,
    /// The ideas whose press is in flight.
    busy: HashSet<String>,
    /// Why a press did not come back done.
    refused: Option<SharedString>,
    /// The clock the ages read against: the walk's 13:05, else this Mac's.
    now: Option<chrono::DateTime<chrono::Local>>,
}

impl ParkedIdeas {
    pub fn new(now: Option<chrono::DateTime<chrono::Local>>) -> Self {
        ParkedIdeas { ideas: None, busy: HashSet::new(), refused: None, now }
    }

    pub fn set_clock(&mut self, now: Option<chrono::DateTime<chrono::Local>>) {
        self.now = now;
    }

    /// What Fleet keeps now.
    pub fn set(&mut self, ideas: Vec<ParkedIdea>, cx: &mut Context<Self>) {
        self.busy.retain(|id| ideas.iter().any(|idea| &idea.id == id));
        self.ideas = Some(ideas);
        cx.notify();
    }

    /// Newest first, which is not the order Fleet keeps them in.
    pub fn ideas(&self) -> Vec<&ParkedIdea> {
        let mut ideas: Vec<&ParkedIdea> = self.ideas.iter().flatten().collect();
        ideas.sort_by(|a, b| at_of(b).cmp(&at_of(a)));
        ideas
    }

    /// Whether there is anything to draw: ideas, or rows where they will be.
    pub fn draws(&self) -> bool {
        self.ideas.as_ref().is_none_or(|ideas| !ideas.is_empty())
    }

    pub fn is_read(&self) -> bool {
        self.ideas.is_some()
    }

    /// A press is out for the idea; or it is back.
    pub fn busy(&mut self, id: &str, on: bool, cx: &mut Context<Self>) {
        if on {
            self.busy.insert(id.to_string());
            self.refused = None;
        } else {
            self.busy.remove(id);
        }
        cx.notify();
    }

    pub fn refuse(&mut self, why: String, cx: &mut Context<Self>) {
        self.refused = Some(why.into());
        cx.notify();
    }

    fn row(&self, idea: &ParkedIdea, look: &Look, cx: &Context<Self>) -> AnyElement {
        let words = words_of(idea);
        let cut = crate::toasts::about(&words);
        let busy = self.busy.contains(&idea.id);
        let now = self.now.unwrap_or_else(chrono::Local::now);
        let age = crate::job_view::age(&idea.at, now);
        let clock = super::view::minute(&idea.at, now);
        let kind = idea.dispatch.kind.map(|kind| {
            let face = dispatch_kind(kind);
            chip(face.word, look.color(face.hue), look.color(face.ground), look).child(glyph(face.glyph, look.length(token::SPACE_3), look.color(face.hue)))
        });
        // The acts stand in fixed columns, so they line up down the list at any width; the words take what is left.
        let column = |width: f32| div().flex_none().w(look.length(token::SPACE_12) * width).flex().items_center().justify_end();
        let start = {
            let idea = idea.clone();
            let button = press(format!("start-{}", idea.id), if busy { "Starting".into() } else { "Start".into() }, None, None, Weight::Plain, look)
                .tooltip(tip("Send it to the Operator as a dispatch"))
                .when(busy, |button| button.opacity(0.5));
            let name = format!("Start {cut}");
            labelled(if busy { button } else { pressed(button, cx, move |_, _, cx| cx.emit(ParkedAsked::Start(idea.clone()))) }, Role::Button, name)
        };
        let remove = {
            let id = idea.id.clone();
            let button = press(format!("remove-{}", idea.id), "Remove".into(), None, None, Weight::Bare, look).tooltip(tip("Let it go")).when(busy, |button| button.opacity(0.5));
            let name = format!("Remove {cut}");
            labelled(if busy { button } else { pressed(button, cx, move |_, _, cx| cx.emit(ParkedAsked::Remove(id.clone()))) }, Role::Button, name)
        };
        // Under the words: what he called it, where he called it anything, and how long ago.
        let under = (kind.is_some() || age.is_some()).then(|| {
            div()
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_2))
                .text_size(look.length(token::TEXT_XS))
                .text_color(look.color(token::FG_SUBTLE))
                .children(kind)
                .children(age.clone().map(|age| div().id(SharedString::from(format!("idea-age-{}", idea.id))).when_some(clock, |age, clock| age.tooltip(tip(clock))).child(age)))
        });
        labelled(
            div()
                .id(SharedString::from(format!("idea-{}", idea.id)))
                .flex()
                .items_center()
                .gap(look.length(token::SPACE_3))
                .px(look.length(token::SPACE_4))
                .py(look.length(token::SPACE_2))
                .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
                .text_size(look.length(token::TEXT_SM))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_1)).child(div().line_clamp(2).child(words.clone())).children(under))
                .child(column(1.9).child(start))
                .child(column(1.9).child(remove)),
            Role::ListItem,
            words,
        )
        .into_any_element()
    }

    fn bones(look: &Look) -> Vec<AnyElement> {
        (0..2)
            .map(|at| {
                div()
                    .px(look.length(token::SPACE_4))
                    .py(look.length(token::SPACE_2))
                    .min_h(look.length(token::H_CONTROL_LG) + look.length(token::SPACE_3))
                    .flex()
                    .items_center()
                    .child(div().h(look.length(token::LEADING_SM)).w(gpui::relative(if at == 0 { 0.6 } else { 0.4 })).rounded(look.length(token::RADIUS_LG)).bg(look.color(token::BG_HOVER)))
                    .into_any_element()
            })
            .collect()
    }
}

/// What he sent: the request, else the words of the message that went with it.
fn words_of(idea: &ParkedIdea) -> String {
    idea.dispatch
        .start
        .request
        .clone()
        .or_else(|| {
            idea.dispatch.then.iter().find_map(|then| match then {
                ipc::DispatchThen::Message(message) => Some(message.text.clone()),
                ipc::DispatchThen::Notes(_) => None,
            })
        })
        .unwrap_or_default()
}

fn at_of(idea: &ParkedIdea) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(&idea.at).map_or(chrono::DateTime::<chrono::Utc>::MIN_UTC, |at| at.to_utc())
}

impl Render for ParkedIdeas {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let look = cx.look();
        if !self.draws() {
            return div().into_any_element();
        }
        let rows: Vec<AnyElement> = match &self.ideas {
            Some(_) => self.ideas().into_iter().map(|idea| self.row(idea, look, cx)).collect(),
            None => Self::bones(look),
        };
        let refused = self.refused.clone().map(|why| {
            labelled(div().id("ideas-refused").text_size(look.length(token::TEXT_SM)).text_color(look.color(token::STATUS_COMPLETED_FAILED)).child(why.clone()), Role::Alert, why)
        });
        let list = div()
            .id("ideas")
            .flex_none()
            .flex()
            .flex_col()
            .rounded(look.length(token::RADIUS_LG) * 1.5)
            .border(look.length(token::BORDER_WIDTH))
            .border_color(look.color(token::BORDER_DEFAULT))
            .overflow_hidden()
            // A rule between rows, never under the last: a border there runs square past the corner.
            .children(rows.into_iter().enumerate().flat_map(|(at, row)| {
                let rule = (at > 0).then(|| div().flex_none().h(look.length(token::BORDER_WIDTH)).bg(look.color(token::BORDER_SUBTLE)).into_any_element());
                rule.into_iter().chain(std::iter::once(row))
            }));
        div()
            .flex()
            .flex_col()
            .gap(look.length(token::SPACE_3))
            .font_family(sans(look))
            .child(heading(look))
            .child(labelled(list, Role::List, "Ideas"))
            .children(refused)
            .into_any_element()
    }
}

fn heading(look: &Look) -> Div {
    div().flex().items_center().child(labelled(
        div().id("ideas-heading").text_size(look.length(token::TEXT_BASE)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).child("Ideas"),
        Role::Heading,
        "Ideas",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idea(id: &str, at: &str, words: &str) -> ParkedIdea {
        let wire = format!(r#"{{"id":"{id}","at":"{at}","dispatch":{{"manifest_id":"armada","request":"{words}"}}}}"#);
        ipc::decode("a parked idea", wire.as_bytes()).expect("a parked idea")
    }

    #[test]
    fn ideas_come_newest_first() {
        let mut list = ParkedIdeas::new(None);
        list.ideas = Some(vec![idea("a", "2026-11-11T09:00:00-05:00", "first"), idea("c", "2026-11-11T12:00:00-05:00", "third"), idea("b", "2026-11-11T10:00:00-05:00", "second")]);
        let order: Vec<_> = list.ideas().iter().map(|idea| idea.id.as_str()).collect();
        assert_eq!(order, ["c", "b", "a"]);
    }

    #[test]
    fn a_list_not_read_draws_rows_and_an_empty_one_draws_nothing() {
        let mut list = ParkedIdeas::new(None);
        assert!(list.draws(), "rows stand where the words will be while it is read");
        list.ideas = Some(Vec::new());
        assert!(!list.draws());
        list.ideas = Some(vec![idea("a", "2026-11-11T09:00:00-05:00", "first")]);
        assert!(list.draws());
    }

    #[test]
    fn an_idea_reads_as_its_request_else_the_message_that_went_with_it() {
        assert_eq!(words_of(&idea("a", "2026-11-11T09:00:00-05:00", "Keep the scrollback")), "Keep the scrollback");
        let wire = r#"{"id":"b","at":"2026-11-11T09:00:00-05:00","dispatch":{"manifest_id":"armada","then":[{"message":{"text":"From the message"}}]}}"#;
        assert_eq!(words_of(&ipc::decode("a parked idea", wire.as_bytes()).expect("a parked idea")), "From the message");
    }
}
