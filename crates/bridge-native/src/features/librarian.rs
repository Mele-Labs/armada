//! The librarian's card over the Features list: where a project's librarian stands, and the press that runs it.
//!
//! What it says is worked out here from Fleet's `LibrarianState` and where a press is (`card`), and nothing there touches the
//! screen. The states Fleet can say today are drawn: never read, current, behind, reading and refused. The ones that wait on
//! Fleet's later parts (features waiting for him, a doc and the code that disagree, the night's share spent, a source it cannot
//! reach) are not drawn, and the card is a column of lines, so each takes a line of its own where it comes.

use chrono::{DateTime, Utc};
use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString, Task};
use ipc::LibrarianState;

use super::dossier::{button, tint};
use super::{FeaturesScreen, Load};
use crate::labels::labelled;
use crate::look::{token, ActiveLook, Look, Token};
use crate::reviews::card_inner_radius;
use crate::settings::view::{refusal, tip};
use crate::settings::wire::Refused;
use crate::titlebar::parts::{glyph, spinner};

/// Where a press stands, kept apart from what Fleet says so a read of the library does not undo it.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Press {
    #[default]
    Idle,
    /// The act is in flight.
    Working,
    /// Fleet refused it, in its own words.
    Failed(String),
}

/// What the card says.
#[derive(Clone, Debug, PartialEq)]
pub enum Card {
    Never,
    Current { through: Option<u64>, when: String },
    Behind { since: Option<u64>, merged: Vec<u64> },
    Working,
    Failed(String),
}

/// How long ago `at` was: `just now`, `4 min ago`, `2 h ago`, `3 d ago`. Empty where `at` is no instant.
pub fn ago(at: &str, now: DateTime<Utc>) -> String {
    let Ok(then) = DateTime::parse_from_rfc3339(at) else { return String::new() };
    let minutes = (now - then.with_timezone(&Utc)).num_minutes().max(0);
    match minutes {
        0 => "just now".into(),
        1..=59 => format!("{minutes} min ago"),
        60..=1439 => format!("{} h ago", minutes / 60),
        _ => format!("{} d ago", minutes / 1440),
    }
}

pub fn card(state: &LibrarianState, press: &Press, now: DateTime<Utc>) -> Card {
    match press {
        Press::Working => return Card::Working,
        Press::Failed(why) => return Card::Failed(why.clone()),
        Press::Idle => {}
    }
    let Some(ran) = &state.last_run else { return Card::Never };
    if state.merged_since.is_empty() {
        Card::Current { through: state.read_through, when: ago(ran.as_str(), now) }
    } else {
        Card::Behind { since: state.read_through, merged: state.merged_since.clone() }
    }
}

impl Card {
    /// The line at the top of the card.
    pub fn said(&self) -> String {
        match self {
            Card::Never => "Not read yet".into(),
            Card::Current { through, when } => {
                let head = through.map_or_else(|| "Read".to_string(), |n| format!("Read through #{n}"));
                if when.is_empty() {
                    head
                } else {
                    format!("{head}, {when}")
                }
            }
            Card::Behind { since: Some(n), .. } => format!("Not read since #{n}"),
            Card::Behind { since: None, .. } => "Merged since the last read".into(),
            Card::Working => "Reading".into(),
            Card::Failed(why) => why.clone(),
        }
    }

    /// The pull requests merged since the last read, oldest first, as one run of numbers.
    pub fn merged_said(&self) -> Option<String> {
        match self {
            Card::Behind { merged, .. } => Some(merged.iter().map(|n| format!("#{n}")).collect::<Vec<_>>().join(" ")),
            _ => None,
        }
    }

    /// The button the card carries: its words, whether it is the primary one, and whether it is off. Current has none.
    pub fn button(&self) -> Option<(&'static str, bool, bool)> {
        match self {
            Card::Never => Some(("Catch up", false, false)),
            Card::Behind { .. } => Some(("Catch up", true, false)),
            Card::Working => Some(("Catch up", false, true)),
            Card::Failed(_) => Some(("Retry", false, false)),
            Card::Current { .. } => None,
        }
    }

    /// Whether a press runs the librarian: the same as the button being there and on.
    pub fn presses(&self) -> bool {
        self.button().is_some_and(|(_, _, off)| !off)
    }

    /// The hue of the edge band and the glyph, the glyph's name and what its tooltip says.
    fn mark(&self) -> (Token, &'static str, &'static str) {
        match self {
            Card::Never => (token::FG_SUBTLE, "circle-dashed", "The librarian has not read this project"),
            Card::Current { .. } => (token::STATUS_COMPLETED_SUCCESS, "circle-check", "Read through the last merged pull request"),
            Card::Behind { .. } => (token::STATUS_AWAITING_REVIEW, "clock-alert", "Pull requests merged since the last read"),
            Card::Working => (token::STATUS_RUNNING, "loader-circle", "Reading"),
            Card::Failed(_) => (token::STATUS_COMPLETED_FAILED, "circle-alert", "Fleet refused the run"),
        }
    }
}

impl FeaturesScreen {
    /// The card for the library on screen, where one is.
    pub(super) fn librarian_card(&self) -> Option<Card> {
        match &self.library {
            Load::Read(library) => Some(card(&library.librarian, &self.press, Utc::now())),
            _ => None,
        }
    }

    /// The card over the list (`.lib`): a band down its left edge and the glyph in the state's hue, the librarian's name, what it
    /// says, the merged numbers where it is behind, and the button.
    pub(super) fn render_librarian(&self, look: &Look, cx: &mut Context<Self>) -> Option<AnyElement> {
        let card = self.librarian_card()?;
        let (hue, name, why) = card.mark();
        let ink = look.color(hue);
        let size = look.length(token::SPACE_4);
        let mark = match card {
            Card::Working => spinner("features-librarian-spin", size, ink, cx).into_any_element(),
            _ => glyph(name, size, ink).into_any_element(),
        };
        let mark = labelled(div().id("features-librarian-state").flex_none().tooltip(tip(why)).child(mark), Role::Image, why);
        let head = div()
            .flex()
            .items_center()
            .gap(look.length(token::SPACE_2))
            .child(mark)
            .child(div().text_size(look.length(token::TEXT_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child("Librarian"));
        let said = card.said();
        let line = match &card {
            Card::Failed(_) => refusal(look, said.clone()),
            _ => div().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).text_color(look.color(token::FG_MUTED)).child(SharedString::from(said.clone())),
        };
        let line = labelled(div().id("features-librarian-said").child(line), Role::Label, said);
        let merged = card.merged_said().map(|numbers| {
            labelled(
                div()
                    .id("features-librarian-merged")
                    .max_h(look.length(token::LEADING_2XS) * 2.)
                    .overflow_hidden()
                    .text_size(look.length(token::TEXT_2XS))
                    .line_height(look.length(token::LEADING_2XS))
                    .text_color(look.color(token::FG_SUBTLE))
                    .child(SharedString::from(numbers.clone())),
                Role::Label,
                format!("Merged since: {numbers}"),
            )
        });
        let press = card.button().map(|(words, primary, off)| {
            let pill = button(look, "features-librarian-press", words, primary, false, off, None);
            let pill = if off { pill } else { pill.on_click(cx.listener(|this, _, _, cx| this.catch_up(cx))) };
            div().flex().child(labelled(pill, Role::Button, words))
        });
        let body = div().flex_1().min_w_0().flex().flex_col().gap(look.length(token::SPACE_2)).p(look.length(token::SPACE_3)).child(head).child(line).children(merged).children(press);
        let band = div().flex_none().w(look.length(token::SPACE_1)).rounded_l(card_inner_radius(look)).bg(ink);
        let ground = tint(look, look.color(token::BG_SUNKEN), hue, 0.06);
        let edge = tint(look, look.color(token::BORDER_SUBTLE), hue, 0.30);
        let shell = div()
            .id("features-librarian")
            .flex_none()
            .mx(look.length(token::SPACE_3))
            .mb(look.length(token::SPACE_3))
            .flex()
            .rounded(look.length(token::RADIUS_LG))
            .border(look.length(token::BORDER_WIDTH))
            .border_color(edge)
            .bg(ground)
            .overflow_hidden()
            .child(band)
            .child(body);
        Some(labelled(shell, Role::Region, "Librarian").into_any_element())
    }
}

/// How long the mock's run takes, so a walk and a test can see it reading.
const MOCK_READS: std::time::Duration = std::time::Duration::from_millis(600);

impl FeaturesScreen {
    /// Catch up, or Retry, or `c`: run the project's librarian now. Nothing happens while one is running or where it has read
    /// through the last merge.
    pub(super) fn catch_up(&mut self, cx: &mut Context<Self>) {
        if !self.librarian_card().is_some_and(|card| card.presses()) {
            return;
        }
        let Some(project) = self.library_project.clone() else { return };
        let ask = match &self.source {
            super::Source::Live(fleet) => match fleet.read(cx).held().port() {
                Some(port) => super::wire::run_librarian(port, &project, cx),
                None => Task::ready(Err(Refused::said("Fleet is not connected"))),
            },
            super::Source::Mock => {
                let refused = self.mock_refusal.clone();
                let through = match &self.library {
                    Load::Read(library) => library.librarian.merged_since.last().copied().or(library.librarian.read_through),
                    _ => None,
                };
                let (executor, project) = (cx.background_executor().clone(), project.clone());
                cx.background_spawn(async move {
                    executor.timer(MOCK_READS).await;
                    match refused {
                        Some(said) => Err(Refused::said(said)),
                        None => Ok(ipc::LibrarianRan { project, ran_at: ipc::Instant::carried(Utc::now().to_rfc3339()), ran: Vec::new(), read_through: through }),
                    }
                })
            }
        };
        self.press = Press::Working;
        self.ran = Some(cx.spawn(async move |this, cx| {
            let answer = ask.await;
            let _ = this.update(cx, |this, cx| this.ran_took(&project, answer, cx));
        }));
        cx.notify();
    }

    /// The press came back: what Fleet said it read through is the card's now, with nothing behind it, and Fleet is read again for the truth.
    fn ran_took(&mut self, project: &str, answer: Result<ipc::LibrarianRan, Refused>, cx: &mut Context<Self>) {
        if self.library_project.as_deref() != Some(project) {
            return;
        }
        match answer {
            Ok(ran) => {
                if let Load::Read(library) = &mut self.library {
                    let state = &mut library.librarian;
                    state.read_through = ran.read_through.or(state.read_through);
                    state.last_run = Some(ran.ran_at);
                    state.merged_since.clear();
                }
                self.press = Press::Idle;
                if matches!(self.source, super::Source::Live(_)) && self.entered {
                    self.read(cx);
                }
            }
            Err(why) => self.press = Press::Failed(why.said),
        }
        cx.notify();
    }
}
