//! The Judge's call put about a Session's work rather than a Job's step. `docs/concepts/judge.md`,
//! *A Session's judge*.
//!
//! **The same call, the same answer format and the same reading.** A brief is laid in labelled parts so
//! a citation says where it landed, an answer that quotes words which are not in the brief is
//! discarded, and a refusal has to say what was expected, what was produced and what follows.
//! [`Brief::read`] reads it back unchanged.
//!
//! **What it is shown is `Work`'s fields and nothing else**: the request the person dispatched, the
//! plan, the diff against the base. There is no parameter for the transcript, so a Session's judge
//! cannot read the agent's account of its own work, as a Job's cannot. What differs from a Job's
//! brief is that there is no step to take a product from: the repository's rules, the request, the
//! plan and the diff are given, and the question is the criterion's.

use core_model::JudgeCriterion;

use crate::judge::{Brief, JUDGE_ANSWER, JUDGE_OPENING, JUDGE_QUESTION};
use crate::shown::Laid;
use crate::standing::Standing;
use crate::wording::Wording;

/// What a Session's judge is shown.
#[derive(Clone, Copy, Debug)]
pub struct Work<'a> {
    /// Where the Session stands: a step's label, or what the person asked a reading of.
    pub label: &'a str,
    /// The words the person dispatched with.
    pub request: &'a str,
    /// The plan as the agent recorded it, with the state Fleet observed for each task. Empty where
    /// the Session has recorded none.
    pub plan: &'a str,
    /// The Session's diff against its base, as git prints it. Empty where nothing changed.
    pub diff: &'a str,
}

const REQUEST: &str = "What the person asked for, in his own words. It is the standard the work \
below is measured against, and not something under judgment:";

const PLAN: &str = "The plan the agent recorded for the work, with how far Fleet observed each \
task carried out. It says what the work set out to do:";

const NO_PLAN: &str = "The agent has recorded no plan for this work.";

const DIFF: &str = "The work so far, as a diff against the branch it will land on:";

const NO_DIFF: &str = "No file has changed yet.";

/// What a Session's judge is told beside the answer format: its refusal is read in three short parts,
/// by the person, and not as a report. A Job's judge is not told this.
const BREVITY: &str = "Write each of the three lines as one short sentence. Do not restate the \
task, the plan or the diff: say only what is missing or wrong.";

impl Brief {
    /// Assemble the question for one criterion about a Session's work.
    pub fn about_work(
        work: &Work<'_>,
        criterion: &JudgeCriterion,
        standing: &Standing,
        wording: &Wording,
    ) -> Brief {
        let mut laid = Laid::new();
        laid.loose(wording.get(JUDGE_OPENING));
        laid.loose("\n\n");
        laid.loose(&format!("Work: {}\n\n", work.label));
        laid.part("request", &format!("{REQUEST}\n\n  {}\n\n", work.request.trim()));
        laid.part("standing", &standing.told(wording));
        let plan = work.plan.trim_end();
        laid.part(
            "plan",
            &match plan.is_empty() {
                true => format!("{NO_PLAN}\n\n"),
                false => format!("{PLAN}\n\n{plan}\n\n"),
            },
        );
        let diff = work.diff.trim_end();
        laid.part(
            "diff",
            &match diff.is_empty() {
                true => format!("{NO_DIFF}\n\n"),
                false => format!("{DIFF}\n\n{diff}\n\n"),
            },
        );
        laid.loose(wording.get(JUDGE_QUESTION));
        laid.loose("\n\n");
        laid.loose(&criterion.question);
        laid.loose("\n\n");
        laid.loose(wording.get(JUDGE_ANSWER));
        laid.loose("\n\n");
        laid.loose(BREVITY);
        let (question, regions) = laid.done();
        Brief { criterion: criterion.criterion_id.clone(), question, regions, asks_risk: false }
    }
}
