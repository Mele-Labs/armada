//! The steps Catch up takes, listed on the librarian's card before a press and ticked off as the run goes.
//!
//! What a row says is worked out here from Fleet's plan (`LibrarianStep`) and what it has told of the run (`librarian.step`),
//! and nothing here touches the screen but `render_steps`. `docs/concepts/feature-catalog.md`, *Librarian*.

use gpui::{div, prelude::*, AnyElement, Context, FontWeight, Role, SharedString};
use ipc::{LibrarianRunning, LibrarianStep, LibrarianStepMoved, LibrarianStepNow, LibrarianStepState};

use super::dossier::pill;
use super::FeaturesScreen;
use crate::labels::labelled;
use crate::look::{token, Look, Token};
use crate::titlebar::parts::{glyph, spinner};

/// A run on screen: the plan it started with and what has been told of each step.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub project: String,
    pub run: ipc::Instant,
    pub plan: Vec<LibrarianStep>,
    pub steps: Vec<LibrarianStepNow>,
    /// The press that started it came back, or a read of the library found none going.
    pub finished: bool,
}

/// Where one row stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stand {
    /// Not reached yet, or no run at all.
    Waiting,
    Running,
    Done,
    Skipped,
    Failed,
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub title: String,
    pub model: bool,
    pub stand: Stand,
    /// What the row says under its title: what the step said where it ended, else what it will do.
    pub text: String,
    /// Waiting while a run goes on: drawn at half.
    pub dim: bool,
}

impl Row {
    /// The name VoiceOver reads and a walk aims at.
    pub fn name(&self) -> String {
        let kind = if self.model { "model" } else { "free" };
        let stand = match self.stand {
            Stand::Waiting if self.dim => "to do",
            Stand::Waiting => "",
            Stand::Running => "running",
            Stand::Done => "done",
            Stand::Skipped => "skipped",
            Stand::Failed => "failed",
        };
        let head = if stand.is_empty() { format!("{}, {kind}", self.title) } else { format!("{}, {kind}, {stand}", self.title) };
        if self.text.is_empty() {
            head
        } else {
            format!("{head}: {}", self.text)
        }
    }
}

impl Run {
    /// A run Fleet says is going, as a read of the library tells it.
    pub fn read(project: &str, plan: &[LibrarianStep], running: &LibrarianRunning) -> Run {
        Run { project: project.to_string(), run: running.run.clone(), plan: plan.to_vec(), steps: running.steps.clone(), finished: false }
    }

    /// A run first heard of by a step moving.
    pub fn begun(project: &str, plan: &[LibrarianStep], moved: &LibrarianStepMoved) -> Run {
        let mut run = Run { project: project.to_string(), run: moved.run.clone(), plan: plan.to_vec(), steps: Vec::new(), finished: false };
        run.moved(moved);
        run
    }

    /// A step started or ended: the latest word on it replaces the one held.
    pub fn moved(&mut self, moved: &LibrarianStepMoved) {
        let now = LibrarianStepNow { step: moved.step.clone(), state: moved.state, said: moved.said.clone() };
        match self.steps.iter_mut().find(|held| held.step == moved.step) {
            Some(held) => *held = now,
            None => self.steps.push(now),
        }
    }

    fn of(&self, id: &str) -> Option<&LibrarianStepNow> {
        self.steps.iter().find(|held| held.step == id)
    }

    /// Every step of the plan has ended.
    pub fn ended(&self) -> bool {
        !self.plan.is_empty() && self.plan.iter().all(|step| self.of(&step.id).is_some_and(|now| now.state != LibrarianStepState::Running))
    }

    /// A run is going on now.
    pub fn going(&self) -> bool {
        !self.finished && !self.ended()
    }

    /// The press came back: the steps it never reached were left out, and stop is why where it was pressed.
    pub fn close(&mut self, stopped: bool) {
        self.finished = true;
        if stopped {
            let left: Vec<String> = self.plan.iter().filter(|step| self.of(&step.id).is_none_or(|now| now.state == LibrarianStepState::Running)).map(|step| step.id.clone()).collect();
            for id in left {
                self.steps.retain(|held| held.step != id);
                self.steps.push(LibrarianStepNow { step: id, state: LibrarianStepState::Skipped, said: "Stopped".into() });
            }
        }
    }

    /// The rows: a run's own plan, with what each step has said.
    pub fn rows(&self) -> Vec<Row> {
        rows(&self.plan, Some(self))
    }
}

/// The rows for `plan`, with a run's word on each step where one is shown.
pub fn rows(plan: &[LibrarianStep], run: Option<&Run>) -> Vec<Row> {
    let going = run.is_some_and(Run::going);
    plan.iter()
        .map(|step| {
            let now = run.and_then(|run| run.of(&step.id));
            let (stand, said) = match now {
                Some(now) => (
                    match now.state {
                        LibrarianStepState::Running => Stand::Running,
                        LibrarianStepState::Done => Stand::Done,
                        LibrarianStepState::Skipped => Stand::Skipped,
                        LibrarianStepState::Failed => Stand::Failed,
                    },
                    now.said.as_str(),
                ),
                None => (Stand::Waiting, ""),
            };
            let text = if said.is_empty() { step.detail.clone() } else { said.to_string() };
            Row { id: step.id.clone(), title: step.title.clone(), model: step.model, stand, text, dim: going && stand == Stand::Waiting }
        })
        .collect()
}

impl FeaturesScreen {
    /// A step of a run moved (`librarian.step`): a move for the project on screen is folded into the run shown, and a move of
    /// another run replaces it. When the last step ends the library is read again.
    pub(super) fn step_moved(&mut self, moved: &LibrarianStepMoved, cx: &mut Context<Self>) {
        if self.library_project.as_deref() != Some(moved.project.as_str()) {
            return;
        }
        match &mut self.run {
            Some(run) if run.run == moved.run && run.project == moved.project => run.moved(moved),
            _ => {
                let plan = match &self.library {
                    super::Load::Read(library) => library.librarian.steps.clone(),
                    _ => Vec::new(),
                };
                self.run = Some(Run::begun(&moved.project, &plan, moved));
            }
        }
        if self.run.as_ref().is_some_and(Run::ended) && matches!(self.source, super::Source::Live(_)) && self.entered {
            self.read(cx);
        }
        cx.notify();
    }

    /// A library was read: a run it says is going replaces one that is not the same, and none going ends one this screen thought was.
    pub(super) fn run_read(&mut self, library: &ipc::FeatureLibrary) {
        match (&library.librarian.running, &mut self.run) {
            (Some(running), Some(run)) if run.run == running.run && run.project == library.project => {
                // What the screen heard stands; a step it missed is filled from the read.
                for now in &running.steps {
                    if run.of(&now.step).is_none() {
                        run.steps.push(now.clone());
                    }
                }
            }
            (Some(running), _) => self.run = Some(Run::read(&library.project, &library.librarian.steps, running)),
            (None, Some(run)) if self.press == super::librarian::Press::Idle => run.finished = true,
            (None, _) => {}
        }
    }

    /// The rows the card lists now, none where it lists none.
    pub(super) fn step_rows(&self, listing: bool) -> Vec<Row> {
        match (&self.run, &self.library) {
            (Some(run), _) => run.rows(),
            (None, super::Load::Read(library)) if listing => rows(&library.librarian.steps, None),
            _ => Vec::new(),
        }
    }
}

/// The ring or mark at the head of a row.
fn check(look: &Look, row: &Row, cx: &gpui::App) -> AnyElement {
    let side = look.length(token::SPACE_4);
    let disc = |hue: Token| div().flex_none().size(side).rounded(side / 2.).flex().items_center().justify_center().bg(look.color(hue));
    let ring = |hue: Token| div().flex_none().size(side).rounded(side / 2.).flex().items_center().justify_center().border(look.length(token::BORDER_WIDTH) * 1.5).border_color(look.color(hue));
    let small = look.length(token::SPACE_3);
    match row.stand {
        Stand::Waiting => ring(token::BORDER_STRONG).into_any_element(),
        Stand::Running => div().flex_none().size(side).flex().items_center().justify_center().child(spinner(SharedString::from(format!("features-librarian-step-spin-{}", row.id)), side, look.color(token::STATUS_RUNNING), cx)).into_any_element(),
        Stand::Done => disc(token::STATUS_COMPLETED_SUCCESS).child(glyph("check", small, look.color(token::FG_INVERSE))).into_any_element(),
        Stand::Skipped => ring(token::BORDER_DEFAULT).child(glyph("minus", small, look.color(token::FG_SUBTLE))).into_any_element(),
        Stand::Failed => disc(token::STATUS_COMPLETED_FAILED).child(glyph("x", small, look.color(token::FG_INVERSE))).into_any_element(),
    }
}

/// What a row says under its title, in the hue its state takes.
fn said_colour(row: &Row) -> Token {
    match row.stand {
        Stand::Failed => token::STATUS_COMPLETED_FAILED,
        _ => token::FG_SUBTLE,
    }
}

/// The ordered list under the card's line: one row for each step.
pub(super) fn render_steps(look: &Look, rows: Vec<Row>, cx: &mut Context<FeaturesScreen>) -> AnyElement {
    let items = rows.into_iter().map(|row| {
        let tag = if row.model {
            pill(look, token::STATUS_AWAITING_REVIEW, "model")
        } else {
            pill(look, token::FG_SUBTLE, "free").bg(look.color(token::BG_SUNKEN)).border(look.length(token::BORDER_WIDTH)).border_color(look.color(token::BORDER_SUBTLE))
        };
        let title = div().min_w_0().text_size(look.length(token::TEXT_XS)).line_height(look.length(token::LEADING_XS)).font_weight(FontWeight(look.number(token::WEIGHT_HEADING))).text_color(look.color(token::FG_DEFAULT)).child(SharedString::from(row.title.clone()));
        let head = div().flex().flex_wrap().items_center().gap(look.length(token::SPACE_2)).child(title).child(tag.flex_none());
        let text = (!row.text.is_empty()).then(|| div().text_size(look.length(token::TEXT_2XS)).line_height(look.length(token::LEADING_2XS)).text_color(look.color(said_colour(&row))).child(SharedString::from(row.text.clone())));
        let body = div().flex_1().min_w_0().flex().flex_col().child(head).children(text);
        let mark = div().flex_none().h(look.length(token::LEADING_XS)).flex().items_center().child(check(look, &row, cx));
        let name = row.name();
        let line = div().id(SharedString::from(format!("features-librarian-step-{}", row.id))).flex().items_start().gap(look.length(token::SPACE_2)).when(row.dim, |line| line.opacity(0.5)).child(mark).child(body);
        labelled(line, Role::ListItem, name)
    });
    labelled(div().id("features-librarian-steps").flex().flex_col().gap(look.length(token::SPACE_2)).children(items), Role::List, "Catch up steps").into_any_element()
}
