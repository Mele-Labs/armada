//! The terms a step puts to its Drone: what it must deliver, what it must
//! declare, and the one thing it is offered.
//!
//! **Cut out of [`briefing`](mod@crate::briefing) on that module's own
//! sentence** — five blocks "outlive the turn they were written for and are
//! types rather than paragraphs". Four of them are here, and being values is
//! not a shape they happen to have: [`Declaring`] is sent again on the turn a
//! step advances underneath a Drone, [`Redeclaring`] is sent by
//! `crate::scope` when a footprint leaves the declaration, and neither reaches
//! a Drone through an opening brief at all.
//!
//! **The list was wrong about `Stopped`, which is why it stayed.** That one is
//! built per spawn from the store and is consumed by the single turn it was
//! built for — it is the reason a resuming brief differs from a first one,
//! rather than a term this step puts to whoever works it. `Delivering` was
//! missing from the list and is the plainest member of it. [`Capturing`] is
//! the sixth, added by `#790`.
//!
//! **Every one is written from the definition and from nothing else**, which is
//! `briefing`'s narrowing carried across: [`Checking`] names its Checks and
//! not what they run, [`Delivering`] names a path in the Drone's own
//! worktree, and [`Capturing`] names `evidence.run` and never what it runs.
//! [`Splitting`] reads one step further, because what a step's
//! product *becomes* is a fact about the step after it — and names no workflow.
//!
//! Five of the six carry **drafted wording**. Sanctioned copy is
//! `docs/contracts/agent-prompt.md`'s to write and it has none for them yet.

use config::settings as keys;
use core_model::{FrozenWorkflow, RepoPath, ResolvedCheck, ResolvedStep, StepId};

use crate::prompts::Prompts;

mod words;
pub(crate) use words::*;

/// The file a step is asked to write, where it declares one.
///
/// **The path is the step's, not the Drone's.** A step whose product is written
/// used to hand the next step three prose strings, one of which named a file
/// nothing opened — `verification::submission` says outright that nothing routes
/// on `shown_by`. The `artifact_exists` check makes the file the product, and
/// this block is the half of that a Drone can act on: a check nobody was told
/// about is a step that fails on its first attempt every time.
///
/// **It is an instruction, unlike [`Checking`], and it says why.** A Drone that
/// writes its finding somewhere else has done the work and lost it — measured
/// on 2026-08-29, when a Drone wrote a seven-kilobyte plan under
/// `.armada/<job-id>/` and the Judge was handed the summary instead, refusing
/// the step for not naming a root cause that was on page one of a file nothing
/// had opened. **The path is the whole of the fix**, because Fleet reads the
/// file at exactly this path and puts its contents in the Judge's brief. A
/// Drone that writes it anywhere else is not delivering it, and the block says
/// so where [`notekeeping`] could otherwise be read as offering an alternative.
///
/// **It is written from the check's target and from nothing else.** The
/// narrowing this module keeps is that no block is written from a
/// `ResolvedCheck`'s `run` string; a target is a path in the Drone's own
/// worktree, which it can already list.
///
/// **Drafted wording**, like [`Checking`] and [`Redeclaring`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivering(String);

impl Delivering {
    /// The file this step must write, or `None` where it declares none — which
    /// is every step whose product is the diff.
    ///
    /// **One file, and `ResolvedStep::deliverable` is what says so.** A step
    /// declaring two is refused where it is written, so there is no list here
    /// and no "the first one" to be wrong about.
    pub fn at(prompts: &Prompts, step: &ResolvedStep) -> Option<Delivering> {
        let target = step.deliverable()?;
        Some(Delivering(
            prompts.fill(keys::PROMPT_DRONE_DELIVERS, &[("target", target)]),
        ))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// [`Delivering`]'s counterpart on the step it never fires for on its own: a
/// step that records the plan and nothing else declares no `deliverable`, so
/// this fills the gap for spike 6's reason — a tool's own description does
/// not make a Drone call it. **Since `#1006`, this may also fire beside
/// [`Delivering`]**, on a step that keeps its own product and records the
/// plan too — the text says which case it is, rather than always claiming the
/// plan is the whole of what this part produces. **Drafted**, like
/// [`Delivering`] and [`Checking`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingThePlan(String);

impl RecordingThePlan {
    /// `Some` on the step that records the Job's plan — the same switch
    /// `crate::work_plan::plan_grants` reads to grant the tool at all.
    pub fn at(prompts: &Prompts, step: &ResolvedStep) -> Option<RecordingThePlan> {
        if !step.records_plan() {
            return None;
        }
        let text = match step.deliverable().is_some() {
            // Beside a real deliverable, the plan is not the product —
            // `Delivering` already told the Drone what is, and this says
            // there is a second thing to record besides it.
            true => prompts.get(keys::PROMPT_DRONE_RECORDS_PLAN_TOO),
            false => prompts.get(keys::PROMPT_DRONE_RECORDS_PLAN),
        };
        Some(RecordingThePlan(String::from(text)))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a step tells its Drone when it is captured: that `shown_by` names a
/// file `evidence.run` runs, not one it only reads.
///
/// **`#790`.** Capture worked on the issue that introduced it only because a
/// person wrote the spec by hand and pointed the step at it — nothing told
/// the Drone that `shown_by` would be run at all, so a Drone that had not
/// happened to write one produced a diff, a passing suite or a note, and
/// capture recorded nothing.
///
/// **It names `evidence.run` and stops there.** What that command is, and
/// what `evidence.frames` expects to find, is the repository's own decision
/// — `docs/concepts/manifest.md`'s Evidence section, never a medium or a
/// framework this block could name instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capturing(String);

impl Capturing {
    /// The block this step puts to its Drone, or `None` where the step is
    /// not captured.
    ///
    /// **`ResolvedStep::captured` is the only switch.** A step that is not
    /// captured must not gain a paragraph about specs — the ordinary
    /// `shown_by` obligation in the baseline already covers it.
    pub fn at(prompts: &Prompts, step: &ResolvedStep) -> Option<Capturing> {
        if !step.captured() {
            return None;
        }
        Some(Capturing(String::from(
            prompts.get(keys::PROMPT_DRONE_CAPTURED),
        )))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a step tells its Drone about the asked run, where it has Checks to run.
///
/// **A tool nothing points at is the defect this whole capability is about.**
/// Spike 6 measured that a description alone does not make a Drone call a tool
/// — which is why the Evidence obligation is in the baseline, the scope ask is
/// in [`Declaring`], and this offer exists at all.
///
/// **It offers and does not instruct.** A block a Drone reads as a requirement
/// puts every step through a build nobody asked for, at a cost of minutes, so
/// the whole of it is conditional and the last sentence says outright that not
/// calling it is a legitimate way to work.
///
/// **It names no number.** The allowance is Fleet's, named in the refusal a
/// Drone gets once it is spent — `docs/concepts/drone.md` keeps counters out of
/// what a Drone is told, because "two runs left" is a bar to optimise against.
///
/// **It says twice that this is not the gate**, in the two places a Drone could
/// stop reading: a pass here is not a pass, and submitting is still the only
/// way to report. A Drone reading a green asked run as a finished step would have
/// been made worse off by being offered this at all.
///
/// **Drafted wording**, like [`Redeclaring`] and the gaming half of [`Stopped`].
/// Keeping a Drone ignorant of the Checks was never the defence against it
/// satisfying the bar rather than doing the work — `docs/concepts/judge.md` and
/// the gaming patterns are. What the wider rule cost is `crate::asked_run`'s.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checking(String);

impl Checking {
    /// The offer this step makes, or `None` where it has nothing to run.
    ///
    /// **The step's own declared Checks are the switch**, and it is the same
    /// switch `crate::asked_run` refuses on: a step declaring none would answer
    /// every call with a refusal, and a Drone pointed at a tool that refuses it
    /// is a Drone reading a denial as a broken system. That is the defect this
    /// capability exists to close, arriving from the other side.
    ///
    /// **It is also what keeps the list from being empty.** A heading with
    /// nothing under it says a list exists and then withholds it, which is
    /// worse than the unnamed offer; the same emptiness returns `None` here.
    ///
    /// **Named by [`label`](core_model::ResolvedCheck::label), never by
    /// command, and never as a menu.** Unnamed, the offer left a Drone no way
    /// to tell whether a call answered the question it had but to spend one —
    /// the guessing the tool exists to replace, a step earlier. A label is the
    /// word the report comes back carrying, so offer and answer line up row
    /// for row; a `run` string is what `docs/concepts/drone.md` keeps out of
    /// every block. One ask runs all it may ask, because a Drone that could
    /// pick could skip the Check that would have caught its mistake; what the
    /// repository keeps out of asking is named, with where it runs (#849).
    ///
    /// **And the caveat rides only where it is true.** A named list a Drone
    /// acts on is worse than the vague sentence it replaces if what runs is
    /// sometimes less, so a step holding a path-scoped Check says so — on the
    /// same reading `crate::asked_run` skips by, and never in schema words.
    pub fn at(
        prompts: &Prompts,
        workflow: &FrozenWorkflow,
        step: &ResolvedStep,
        allowance: Option<crate::briefing::Allowance>,
    ) -> Option<Checking> {
        if step.mid_step_checks().is_empty() {
            return None;
        }
        let mut block = String::from(prompts.get(keys::PROMPT_DRONE_CHECKS));
        block.push('\n');
        for check in step.checks() {
            block.push_str("\n  - ");
            block.push_str(check.label());
        }
        let whole = match step.mid_step_checks().len() == step.checks().len() {
            true => prompts.get(keys::PROMPT_DRONE_CHECKS_WHOLE_ALL),
            false => prompts.get(keys::PROMPT_DRONE_CHECKS_WHOLE_EXCEPT),
        };
        block.push_str("\n\n");
        block.push_str(&prompts.fill(keys::PROMPT_DRONE_CHECKS_ASK, &[("whole", whole)]));
        if step.checks().iter().any(ResolvedCheck::needs_changed_paths) {
            block.push(' ');
            block.push_str(prompts.get(keys::PROMPT_DRONE_CHECKS_SKIPPED));
        }
        block.push(' ');
        block.push_str(prompts.get(keys::PROMPT_DRONE_CHECKS_WAIT));
        block.push_str(&not_asked(prompts, workflow, step));
        block.push_str("\n\n");
        block.push_str(prompts.get(keys::PROMPT_DRONE_CHECKS_NOT_A_VERDICT));
        // **The number, where the caller knows it.** "There is a limit" and no
        // number is what a part read on 17 Sep before it stopped asking
        // altogether: it could not tell whether spending one early risked
        // running out at the second task of seven, so it spent none and ran
        // the commands by hand. A brief assembled outside a spawn has no
        // allowance to state and says nothing rather than guessing. #1456.
        match allowance.and_then(|allowance| allowance.said(prompts)) {
            Some(said) => block.push_str(&format!(" {said}")),
            None => {
                block.push(' ');
                block.push_str(prompts.get(keys::PROMPT_DRONE_CHECKS_LIMIT));
            }
        }
        Some(Checking(block))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// The checks asking leaves out and where each runs instead, so a clean run
/// does not read as the whole bar. Empty where asking runs every one. #849.
fn not_asked(prompts: &Prompts, workflow: &FrozenWorkflow, step: &ResolvedStep) -> String {
    let named = |runs_at: core_model::RunsAt| -> Vec<&str> {
        step.checks()
            .iter()
            .filter(|check| check.runs_at() == runs_at)
            .map(ResolvedCheck::label)
            .collect()
    };
    let runs = |names: usize| if names == 1 { "it runs" } else { "they run" };
    let mut said = String::new();
    let at_gate = named(core_model::RunsAt::Gate);
    if !at_gate.is_empty() {
        said.push_str("\n\n");
        said.push_str(&prompts.fill(
            keys::PROMPT_DRONE_CHECKS_AT_GATE,
            &[("checks", &quoted(&at_gate)), ("runs", runs(at_gate.len()))],
        ));
    }
    let last = named(core_model::RunsAt::Handoff);
    if !last.is_empty() {
        said.push_str("\n\n");
        said.push_str(&prompts.fill(
            keys::PROMPT_DRONE_CHECKS_AT_HANDOFF,
            &[("checks", &quoted(&last)), ("runs", runs(last.len()))],
        ));
    }
    let later = workflow.held_for_handoff(step.id());
    if !later.is_empty() {
        let one = later.len() == 1;
        said.push_str("\n\n");
        said.push_str(&prompts.fill(
            keys::PROMPT_DRONE_CHECKS_LATER,
            &[
                ("checks", &quoted(&later)),
                ("is", if one { "is" } else { "are" }),
                ("runs", if one { "It runs" } else { "They run" }),
            ],
        ));
    }
    said
}

/// Names as a reader lists them: `a`, `a` and `b`, `a`, `b` and `c`.
fn quoted(names: &[&str]) -> String {
    let each: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match each.split_last() {
        None => String::new(),
        Some((only, [])) => only.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// What a step asks its Drone to declare before starting, where it asks at all.
///
/// **The obligation is here rather than in the tool's description**, for the
/// reason the baseline carries the Evidence obligation: spike 6 measured that a
/// description alone does not make a Drone call a tool.
///
/// **And it is a value rather than a private paragraph**, because the ask is
/// made more than once. The first turn carries it, and so does the turn a Drone
/// gets when a step advances underneath it — see [`Declaring::at`] for why the
/// second one is not optional.
///
/// The consequence is stated plainly and without a threat: a scope that turns
/// out wrong is fixed by declaring again, and work belonging to a later part
/// does not become this part's by being named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaring(String);

impl Declaring {
    /// The ask this step makes, or `None` where it makes none.
    ///
    /// **`wants_a_declaration` decides it and nothing else does.** That is the
    /// cold switch `core_model::EvidenceScope` describes: a step with no
    /// evidence scope, or one whose context comes from somewhere other than
    /// the Drone, is told exactly what it was told before any of this existed.
    /// Inferring the ask from `declare_plan_at` or from `scope_diff_check`
    /// would put a tool call in front of a Drone that has no tool to make it
    /// with.
    ///
    /// **Every step that wants one is asked, including the fourth step of a
    /// Job whose Drone declared correctly on the first.** One Job declared its
    /// scope on the step that asked for it, advanced, worked the next step for
    /// twenty-two minutes, and failed `evidence_scope` on a call nobody had
    /// requested: the declaration is cleared at the boundary and the ask was
    /// not repeated there.
    pub fn at(prompts: &Prompts, step: &ResolvedStep) -> Option<Declaring> {
        let scope = step.evidence_scope()?;
        if !scope.wants_a_declaration() {
            return None;
        }
        let mut block = String::from(prompts.get(keys::PROMPT_DRONE_DECLARE));
        if scope.scope_diff_check() {
            block.push(' ');
            block.push_str(prompts.get(keys::PROMPT_DRONE_DECLARE_COMPARED));
        }
        if !scope.exclude_paths().is_empty() {
            // **It says there is a route**, and until `#417` it did not. A
            // Drone told only "do not name these" and then finding the fix
            // needs one of them fails its part or works around it, and the
            // person finds out at the end. These are boundaries set before
            // anybody read the code, so whether one is right for this
            // particular fix is a real question with a real answer.
            block.push_str("\n\n");
            block.push_str(prompts.get(keys::PROMPT_DRONE_DECLARE_EXCLUDED));
            for path in scope.exclude_paths() {
                block.push_str("\n  - ");
                block.push_str(path.as_str());
            }
        }
        Some(Declaring(block))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a Drone is told when its work turns up outside the plan it declared.
///
/// **The other half of [`Declaring`], and it was missing.** The live check in
/// `crate::scope` has compared edits against the plan since the scope tool
/// existed, and everything it found went to the Job's log. The Drone was never
/// told, so the one call that fixes a plan that turned out wrong was a call it
/// had no reason to make: One Job drifted onto `crates/ipc/src/lib.rs`,
/// carried on for seven minutes and reached its gate still holding a
/// declaration it had outgrown.
///
/// **It offers rather than demands, and the wording is the whole mechanism.**
/// `docs/concepts/judge.md` keeps drift a signal because legitimate
/// investigation moves the work, so a Drone that reads this as an accusation
/// and apologises, or as a stop-work order and downs tools over a file it was
/// right to touch, has been made worse off by being told. Three sentences
/// carry that: nothing has failed, you are not being asked to stop, and here
/// is the call that makes the plan true. The stop-and-report directive is
/// `crate::converging::ReportNow` and is a different act.
///
/// **Drafted wording**, like the gaming half of [`Stopped`].
/// `docs/contracts/agent-prompt.md` has no sanctioned copy for a mid-step
/// scope notice, and the phrasing here follows [`Declaring`]'s so a Drone
/// reads one vocabulary rather than two.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Redeclaring(String);

impl Redeclaring {
    /// The notice for paths seen outside the plan, or `None` where there is
    /// nothing to say.
    ///
    /// **There is no constructor taking a string**, and the step decides
    /// whether one exists at all — the same narrowing [`Declaring::at`] makes.
    /// `watches_live_edits` is the switch rather than `scope_diff_check`: a
    /// step whose plan arrives at the gate has no live plan to correct, and a
    /// Drone told to call a tool it was never asked to call goes looking for
    /// one.
    ///
    /// **`drifted` is what was just confirmed on a second reading**, which
    /// [`Working::drifting`](crate::working::Working::drifting) already
    /// answers. Passing everything seen so far would say the same thing every
    /// turn, and a notice a Drone has already acted on is one it reads as
    /// having been ignored.
    pub fn at(prompts: &Prompts, step: &ResolvedStep, drifted: &[RepoPath]) -> Option<Redeclaring> {
        if drifted.is_empty() || !step.evidence_scope()?.watches_live_edits() {
            return None;
        }
        let mut block = String::from(prompts.get(keys::PROMPT_DRONE_REDECLARE));
        for path in drifted {
            block.push_str("\n  - ");
            block.push_str(path.as_str());
        }
        block.push_str("\n\n");
        block.push_str(prompts.get(keys::PROMPT_DRONE_REDECLARE_WHAT));
        Some(Redeclaring(block))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// What a step that creates Jobs tells its Drone, and what the step *before*
/// one tells its own.
///
/// **It is the only term read off two steps**, and the pair is the point. A
/// step whose product is a plan and a step that turns that plan into Jobs are
/// one decision written twice: what the first writes is carried out unchanged,
/// and a Drone that does not know that writes a sketch. So the switch is
/// `may_dispatch_jobs` here or on the step after — a fact about the definition
/// in exactly the way [`Delivering`]'s target is.
///
/// **The consequence is what it states, because nothing else can.** A Drone
/// reading `dispatch_job`'s description learns what one call does; it cannot
/// learn there that the file it wrote on the part before is the authority for
/// every call. Spike 6's finding is the general form.
///
/// **It names no notation.** Whether a plan is drawn in Mermaid is the
/// repository's business; what this asks for is a drawing rather than a
/// format, for the reason the owner gave on reading one by hand: a wave is a
/// set of Jobs and an ordering between the sets, and prose asks a reader to
/// redraw it before they can answer.
///
/// **Drafted wording**, like [`Checking`] and [`Redeclaring`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Splitting(String);

impl Splitting {
    /// The block this step puts to its Drone, or `None` on every step that
    /// neither creates Jobs nor stands in front of one that does — which is
    /// every step of every workflow but one.
    pub fn at(prompts: &Prompts, workflow: &FrozenWorkflow, at: &StepId) -> Option<Splitting> {
        if workflow.step(at)?.may_dispatch_jobs() {
            return Some(Splitting(String::from(
                prompts.get(keys::PROMPT_DRONE_SPLIT_PROPOSES),
            )));
        }
        workflow
            .after(at)
            .is_some_and(ResolvedStep::may_dispatch_jobs)
            .then(|| Splitting(String::from(prompts.get(keys::PROMPT_DRONE_SPLIT_DECIDES))))
    }

    /// The block, exactly as it reaches a Drone.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// The step before the one that dispatches: what it writes becomes Jobs.
///
/// **The two reasons a piece waits are told apart** because only one survives
/// leaving the plan: needing what another produced becomes a dependency edge,
/// and two pieces that would write the same files are held apart by the plan
/// and by nothing else — `#47` settled that an overlap is surfaced, never
/// serialised.
pub(crate) const DECIDES_IT: &str = "\
WHAT THIS PART DECIDES

The part after this one creates a Job for each piece you name here. Each of \
them is real: it gets its own worktree, its own agent and its own spend, and \
none of them has read what you read. Nobody rewrites what you write — it is \
carried out as it stands.

So say, for every piece: what it is called, what its agent is to be told, what \
its work is held to, and which pieces must finish before it can start. Draw \
them as well as describing them, because what is being decided is a shape, and \
prose asks whoever reads this to redraw it before they can answer.

Two things make a piece wait and they are not the same thing. One piece needs \
what another produced — that is a fact about the work, and it is carried when \
the Jobs are created. Two pieces would write the same files — that is held \
apart by this plan and by nothing else. Say which you mean.";

/// The dispatching step: what it proposes is real, waits for a person, and is
/// carried out as written. Since slice 6 the step that creates Jobs is the one
/// a person answers, so the decision and the creating are one part.
pub(crate) const PROPOSES_IT: &str = "\
WHAT THIS PART PROPOSES

Each piece you name becomes a Job when you call dispatch_job for it: its own \
worktree, its own agent and its own spend, and none of them has read what you \
read. They wait for a person, who reads every one of them before approving \
them together, and nothing runs until then. Nobody rewrites what you write — \
it is what each piece is told.

So say, for every piece: what it is called, what its agent is to be told, what \
its work is held to, and which pieces must finish before it can start, by the \
ids dispatch_job gave you back. Draw them in the plan as well, because what is \
approved is a shape, and prose asks whoever reads it to redraw it first.

Two things make a piece wait and they are not the same thing. One piece needs \
what another produced — that is a fact about the work, and `after` carries it. \
Two pieces would write the same files — that is held apart by this plan and by \
nothing else. Say which you mean.

If this part runs again, what it proposed before and nobody approved is \
withdrawn, so propose the whole wave again.";
