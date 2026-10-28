//! The claim, written before the code: **a step before the plan ends on any one of the things its
//! `ends_on` lists (a reply, a walk approved, a sketch approved, a draft approved, an idea picked),
//! each read off the thread and never off the agent's word, and a walk approved before the plan was
//! approved never passes the later walk step.** The owner's decision of 28 Oct 2026.

use core_model::StepEnd;
use ipc::{
    AskedChoice, AskedQuestion, DraftHow, DraftSettled, HelmCallAnswer, HelmCallInFlight, Instant, ManifestId,
    SessionAskState, SessionRow, SessionVoice,
};

use super::session_rail::{approved, at, delivers, follows, plan_step, step};
use crate::rail::{self, Facts, RailStep};
use crate::rail_ends::{ends_any, ends_when, End};
use crate::rail_keeping::thread_facts;

// ---- The rules ----

fn ending(label: &str, ends: &[StepEnd]) -> RailStep {
    RailStep { ends_on: ends.to_vec(), instructions: Some("Show the options.".into()), ..step(label) }
}

/// Explore (ends on any of three), Discuss, Approve plan, Walk, Refine, Deliver.
fn explore() -> Vec<RailStep> {
    let walk = RailStep { walks: true, ..step("Walk") };
    vec![
        ending("Explore", &[StepEnd::DraftApproved, StepEnd::IdeaPicked, StepEnd::WalkApproved]),
        step("Discuss"),
        plan_step(),
        walk,
        follows("Refine"),
        delivers("Deliver"),
    ]
}

#[test]
fn a_step_before_the_plan_ends_on_any_one_of_the_things_it_lists() {
    let steps = explore();
    assert_eq!(at(&steps, Facts::default()), Some(0));
    for ended in [
        Facts { draft_approved: true, ..Facts::default() },
        Facts { idea_picked: true, ..Facts::default() },
        Facts { any_walk_approved: true, ..Facts::default() },
    ] {
        assert_eq!(at(&steps, ended), Some(1), "{ended:?} ends Explore");
    }
    assert_eq!(at(&steps, Facts { plan_recorded: true, ..Facts::default() }), Some(2), "a plan recorded still ends it");
}

#[test]
fn only_the_ends_a_step_lists_end_it() {
    let steps = explore();
    assert_eq!(at(&steps, Facts { replied: true, ..Facts::default() }), Some(0), "his answer's message is no listed end");
    assert_eq!(at(&steps, Facts { sketch_approved: true, ..Facts::default() }), Some(0));
    let sketched = [ending("Sketch", &[StepEnd::SketchApproved]), step("Discuss"), plan_step()];
    assert_eq!(at(&sketched, Facts { sketch_approved: true, ..Facts::default() }), Some(1));
    let nothing = [step("Discuss"), plan_step()];
    assert_eq!(at(&nothing, Facts { replied: true, draft_approved: true, ..Facts::default() }), Some(0), "listing none keeps today's rule");
}

#[test]
fn a_walk_approved_before_the_plan_ends_a_step_before_it_and_never_the_walk_step() {
    let steps = explore();
    let early = Facts { any_walk_approved: true, walk_shown: true, plan_recorded: true, ..approved() };
    assert!(!early.walk_passed(), "approved before the plan was");
    assert_eq!(at(&steps, early), Some(3), "the walk step waits for an approval since the plan's");
    assert_eq!(at(&steps, Facts { walk_approved: true, ..early }), Some(4));
}

#[test]
fn every_end_of_a_step_is_said_and_one_alone_is_also_said_as_the_one() {
    let steps = explore();
    assert_eq!(ends_any(&steps, 0), ["you approve a draft", "you pick one of the ideas shown", "you approve a walk"]);
    assert_eq!(ends_when(&steps, 0), None, "several ends are no single phrase");
    assert_eq!(ends_any(&steps, 1), ["a plan is recorded"], "a step that lists none");
    assert_eq!(ends_when(&steps, 2), Some("you approve the plan"));
    let sketched = [ending("Sketch", &[StepEnd::SketchApproved]), plan_step()];
    assert_eq!(ends_when(&sketched, 0), Some("you approve a sketch"));
    for end in [End::AnyWalkApproved, End::SketchApproved, End::DraftApproved, End::IdeaPicked] {
        assert!(!end.by_the_job(), "{end:?} is the person's");
    }
}

#[test]
fn the_agent_is_told_every_end_of_its_step() {
    let said = rail::instructed("explore", &explore(), 0).expect("instructions are sent");
    assert!(
        said.ends_with(
            "This step ends when the person approves a draft you proposed with propose_draft, or the person picks an \
             option you gave a preview in AskUserQuestion, or the person approves a walk you showed with show_window."
        ),
        "{said}"
    );
}

// ---- Read off the thread ----

fn at_second(n: u32) -> Instant {
    Instant::carried(&format!("2026-10-28T09:00:{n:02}.000Z"))
}

fn said(n: u32, id: &str, text: &str) -> SessionRow {
    SessionRow::Message {
        id: id.into(),
        at: at_second(n),
        from: SessionVoice::You,
        text: text.into(),
        files: Vec::new(),
        tags: Vec::new(),
        notice: None,
    }
}

fn window(n: u32, url: &str) -> SessionRow {
    SessionRow::Window {
        id: format!("w{n}"),
        at: at_second(n),
        title: "Mock".into(),
        url: url.into(),
        variations: Vec::new(),
        steps: Vec::new(),
        origins: Vec::new(),
        ended_at: None,
    }
}

#[test]
fn a_walk_counts_for_the_walk_step_only_when_approved_at_or_after_the_plans_approval() {
    let url = "http://localhost:5391/?walk=a";
    let rows = [window(1, url), said(3, "m3", &format!("Approved: {url}"))];
    let before = thread_facts(&rows, Some(&at_second(5)));
    assert!(before.any_walk_approved && !before.walk_approved, "approved before the plan was");
    let after = thread_facts(&rows, Some(&at_second(2)));
    assert!(after.any_walk_approved && after.walk_approved);
    assert!(!thread_facts(&rows, None).walk_approved, "no plan approved, nothing passes the walk step");
}

#[test]
fn a_sketch_is_approved_when_his_approval_of_one_is_on_the_thread_and_not_when_sent_back() {
    let approved = [said(2, "m2", "Approved sketch: Pause on the row\nGo on from it as drawn.")];
    assert!(thread_facts(&approved, None).sketch_approved);
    let sent_back = [said(2, "m2", "Sent back sketch: Pause on the row\nDraw the row without the menu.")];
    assert!(!thread_facts(&sent_back, None).sketch_approved);
}

fn draft(how: Option<DraftHow>) -> SessionRow {
    SessionRow::Draft {
        id: "d1".into(),
        at: at_second(1),
        title: "Release note".into(),
        text: "The reader stops at the end.".into(),
        before: None,
        settled: how.map(|how| DraftSettled { how, words: None, at: at_second(2) }),
    }
}

#[test]
fn a_draft_is_approved_as_written_or_with_his_own_text_and_not_when_sent_back_or_withdrawn() {
    assert!(thread_facts(&[draft(Some(DraftHow::Approved))], None).draft_approved);
    assert!(thread_facts(&[draft(Some(DraftHow::Fixed))], None).draft_approved, "he OK'd the text");
    for how in [None, Some(DraftHow::SentBack), Some(DraftHow::Withdrawn)] {
        assert!(!thread_facts(&[draft(how)], None).draft_approved, "{how:?}");
    }
}

fn choice(label: &str, preview: &str) -> AskedChoice {
    AskedChoice { label: label.into(), description: String::new(), preview: preview.into() }
}

fn question(text: &str, options: Vec<AskedChoice>) -> AskedQuestion {
    AskedQuestion { question: text.into(), header: "Idea".into(), multi_select: false, options }
}

fn ideas() -> AskedQuestion {
    question("Which layout?", vec![choice("Two columns", "| a | b |"), choice("One column", ""), choice("Tabs", "[a] [b]")])
}

fn ask(call: &str, questions: Vec<AskedQuestion>, state: SessionAskState) -> SessionRow {
    SessionRow::Ask {
        id: format!("ask-{call}-s1"),
        at: at_second(1),
        ask: HelmCallInFlight {
            call: call.into(),
            manifest_id: ManifestId::carried("armada"),
            asked_at: at_second(1),
            tool: ipc::ASKS_A_QUESTION.into(),
            detail: String::new(),
            truncated: false,
            length: None,
            whole: None,
            rule: ipc::ASKS_A_QUESTION.into(),
            offers: vec![HelmCallAnswer::AllowOnce, HelmCallAnswer::Refuse],
            holding_for_seconds: 600,
            questions,
            reaches: None,
            reaches_to: None,
            kind: None,
        },
        state,
        via: None,
    }
}

/// The message `person_said` writes for an answer on the card.
fn answered(call: &str, questions: &[(&str, &str)]) -> SessionRow {
    let text = questions.iter().map(|(asked, chosen)| format!("**{asked}**\n{chosen}")).collect::<Vec<_>>().join("\n\n");
    said(2, &format!("ask-{call}-s1:said"), &text)
}

#[test]
fn an_idea_is_picked_by_choosing_an_option_the_question_gave_a_preview() {
    let picked = |chosen: &str| thread_facts(&[ask("c1", vec![ideas()], SessionAskState::AllowedOnce), answered("c1", &[("Which layout?", chosen)])], None);
    assert!(picked("Two columns").idea_picked);
    assert!(picked("Tabs").idea_picked);
    assert!(!picked("One column").idea_picked, "an option without a preview");
    assert!(!picked("Something else entirely").idea_picked, "words of his own");
    let plain = question("Ship it?", vec![choice("Yes", ""), choice("No", "")]);
    let rows = [ask("c2", vec![plain], SessionAskState::AllowedOnce), answered("c2", &[("Ship it?", "Yes")])];
    assert!(!thread_facts(&rows, None).idea_picked, "a question with no previews offers no ideas");
}

#[test]
fn fleets_own_questions_never_count_as_an_idea_picked() {
    let remind = question(ipc::HANDOFF_REMIND_QUESTION, vec![choice("450k", "450,000 tokens")]);
    let rows = [ask("c3", vec![remind], SessionAskState::AllowedOnce), answered("c3", &[(ipc::HANDOFF_REMIND_QUESTION, "450k")])];
    assert!(!thread_facts(&rows, None).idea_picked);
}

#[test]
fn an_idea_picked_later_from_needs_you_counts_as_one_picked_on_the_card() {
    let later = |chosen: &str| {
        let text = format!("Answering your earlier question \"Which layout?\": {chosen}");
        thread_facts(&[ask("c4", vec![ideas()], SessionAskState::Unanswered), said(5, "m5", &text)], None).idea_picked
    };
    assert!(later("Tabs"));
    assert!(!later("One column"));
}
