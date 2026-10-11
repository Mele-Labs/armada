//! Whether a refusal goes back to the Drone rather than to a person — #206.
//!
//! **A panel that agreed, on different models, with nobody calling the work
//! risky, inside the retry budget.** All four, on every criterion that refused,
//! or the refusals take the road every refusal took before: a person. That is
//! the owner's decision of 11 Oct 2026,
//! `.claude/decisions/2026-10-11-agreed-refusals-go-back-to-the-drone.md`.
//!
//! | Must hold | Why |
//! |---|---|
//! | every member of the criterion's panel refused | a split is a close call, and a person reads close calls |
//! | the members ran on at least two models | one model asked three times agrees with itself; that is not a second opinion |
//! | no member called the work risky | the Judges' own sense of risk is what still brings a person in |
//! | fewer than [`AGREED_RETRIES`] went back already | past it, redoing the work has stopped working |
//!
//! **A step's panel size defaults to one**, so nothing here fires until a
//! workflow asks for a panel of different models.

use core_model::{CriterionId, JudgeCheck, Judgment, AGREED_RETRIES};
use verification::Refusals;

/// The refusals to send back to the Drone, or `None` where a person answers.
///
/// `tolerated` is read as `looks::fold` reads it: a criterion stood down is
/// not a refusal at all. `sent_back` is how many times this pass already went
/// back on an agreed refusal.
pub(crate) fn agreed(
    judgments: &[Judgment],
    checks: &[JudgeCheck],
    tolerated: &[CriterionId],
    sent_back: u32,
) -> Option<Refusals> {
    if sent_back >= AGREED_RETRIES {
        return None;
    }
    let refusing: Vec<Judgment> = judgments
        .iter()
        .filter(|judgment| judgment.verdict.refuses())
        .filter(|judgment| !tolerated.contains(&judgment.criterion_id))
        .cloned()
        .collect();
    let every_one_agreed = refusing
        .iter()
        .all(|judgment| unanimous(&judgment.criterion_id, judgments, checks));
    match every_one_agreed {
        true => Refusals::among(&refusing),
        false => None,
    }
}

/// Whether every member of the panel that answered `criterion` refused it, on
/// more than one model, and none called the work risky.
///
/// **A criterion no check declares is never agreed**: that is the drift look,
/// which runs no panel and so has no second opinion to agree with.
fn unanimous(criterion: &CriterionId, judgments: &[Judgment], checks: &[JudgeCheck]) -> bool {
    let Some(check) = checks.iter().find(|check| {
        check
            .criteria()
            .iter()
            .any(|c| &c.criterion_id == criterion)
    }) else {
        return false;
    };
    let answers: Vec<&Judgment> = judgments
        .iter()
        .filter(|judgment| &judgment.criterion_id == criterion)
        .collect();
    let mut models: Vec<&str> = Vec::new();
    for answer in &answers {
        // A member whose model was not recorded cannot be counted as a
        // different opinion, so the panel is not one.
        let Some(given) = &answer.given else {
            return false;
        };
        if !models.contains(&given.model.as_str()) {
            models.push(given.model.as_str());
        }
    }
    check.panel_size() > 1
        && answers.len() == check.panel_size() as usize
        && answers.iter().all(|answer| answer.verdict.refuses())
        && answers.iter().all(|answer| answer.risky.is_none())
        && models.len() > 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_model::{Given, JudgeCriterion, JudgeVerdict, ModelName, OnRefusal};

    fn panel(models: &[&str]) -> Vec<JudgeCheck> {
        let criterion = JudgeCriterion {
            criterion_id: CriterionId::new("c1"),
            question: "Does the fix address the cause?".to_string(),
            on_refusal: OnRefusal::Ask,
        };
        let members = models
            .iter()
            .map(|named| ModelName::new(named).expect("a model"))
            .collect();
        vec![JudgeCheck::declared(None, 1, vec![criterion], None).with_members(members)]
    }

    fn answer(member: u32, model: &str, refuses: bool, risky: Option<&str>) -> Judgment {
        Judgment {
            criterion_id: CriterionId::new("c1"),
            member: Some(member),
            verdict: match refuses {
                true => JudgeVerdict::NotMet,
                false => JudgeVerdict::Met,
            },
            expected: refuses.then(|| "the loop stops at n".to_string()),
            produced: refuses.then(|| "the loop stops at n - 1".to_string()),
            consequence: refuses.then(|| "the last row is dropped".to_string()),
            brief_path: None,
            cited: None,
            given: Some(Given {
                digest: "d".to_string(),
                size: 1,
                model: model.to_string(),
            }),
            risky: risky.map(str::to_string),
        }
    }

    fn both_refuse() -> Vec<Judgment> {
        vec![
            answer(1, "the-cheap-model", true, None),
            answer(2, "the-other-model", true, None),
        ]
    }

    #[test]
    fn a_panel_of_two_models_that_agrees_sends_the_work_back_with_its_reasons() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        let sent = agreed(&both_refuse(), &checks, &[], 0).expect("sent back to the Drone");
        assert_eq!(sent.criteria(), vec![&CriterionId::new("c1"); 2]);
        assert_eq!(
            sent.cited()[0].expected.as_deref(),
            Some("the loop stops at n")
        );
    }

    #[test]
    fn a_split_panel_goes_to_a_person() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        let split = vec![
            answer(1, "the-cheap-model", true, None),
            answer(2, "the-other-model", false, None),
        ];
        assert_eq!(agreed(&split, &checks, &[], 0), None);
    }

    #[test]
    fn one_member_calling_it_risky_brings_a_person_in() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        let risky = vec![
            answer(1, "the-cheap-model", true, None),
            answer(2, "the-other-model", true, Some("it drops a table")),
        ];
        assert_eq!(agreed(&risky, &checks, &[], 0), None);
    }

    #[test]
    fn the_third_agreed_refusal_goes_to_a_person() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        assert!(agreed(&both_refuse(), &checks, &[], 1).is_some());
        assert_eq!(agreed(&both_refuse(), &checks, &[], AGREED_RETRIES), None);
    }

    #[test]
    fn one_model_asked_twice_is_not_a_second_opinion() {
        let checks = vec![JudgeCheck::declared(
            None,
            2,
            panel(&[])[0].criteria().to_vec(),
            None,
        )];
        let same = vec![
            answer(1, "the-cheap-model", true, None),
            answer(2, "the-cheap-model", true, None),
        ];
        assert_eq!(agreed(&same, &checks, &[], 0), None);
    }

    #[test]
    fn a_judge_of_one_is_unchanged() {
        let checks = vec![JudgeCheck::declared(
            None,
            1,
            panel(&[])[0].criteria().to_vec(),
            None,
        )];
        let mut alone = answer(1, "the-cheap-model", true, None);
        alone.member = None;
        assert_eq!(agreed(&[alone], &checks, &[], 0), None);
    }

    #[test]
    fn a_refusal_nobody_declared_is_never_agreed() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        let mut drift = both_refuse();
        drift.push(Judgment {
            criterion_id: CriterionId::new(verification::DECLARED_PLAN_DRIFT),
            ..answer(1, "the-cheap-model", true, None)
        });
        assert_eq!(agreed(&drift, &checks, &[], 0), None);
    }

    #[test]
    fn nothing_refused_sends_nothing_back() {
        let checks = panel(&["the-cheap-model", "the-other-model"]);
        let met = vec![
            answer(1, "the-cheap-model", false, None),
            answer(2, "the-other-model", false, None),
        ];
        assert_eq!(agreed(&met, &checks, &[], 0), None);
    }
}
