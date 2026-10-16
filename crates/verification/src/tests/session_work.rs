//! The Judge's call about a Session's work: what it is shown, what it is not, and that a citation lands
//! in the part that holds it. `docs/concepts/judge.md`, *A Session's judge*.

use core_model::{CriterionId, JudgeCriterion, JudgeVerdict, OnRefusal};

use crate::{Brief, Standing, Unreadable, Wording, Work};

const DIFF: &str = "diff --git a/src/reader.rs b/src/reader.rs\n--- a/src/reader.rs\n+++ b/src/reader.rs\n@@ -1,3 +1,3 @@\n-    rows.take(n)\n+    rows.take(n + 1)\n";

fn criterion() -> JudgeCriterion {
    JudgeCriterion {
        criterion_id: CriterionId::new("on_plan"),
        question: "Does the diff carry out the plan?".to_string(),
        on_refusal: OnRefusal::Ask,
    }
}

fn brief(plan: &str, diff: &str) -> Brief {
    let work = Work { label: "Implement", request: "the reader drops the last row", plan, diff };
    Brief::about_work(&work, &criterion(), &Standing::unstated(), &Wording::shipped())
}

#[test]
fn the_brief_holds_the_request_the_plan_the_diff_and_the_question_and_nothing_of_a_transcript() {
    let said = brief("Fix the bound (open)", DIFF);
    let text = said.question();
    for shown in ["the reader drops the last row", "Fix the bound (open)", "rows.take(n + 1)", "Does the diff carry out the plan?"] {
        assert!(text.contains(shown), "missing {shown:?}: {text}");
    }
    assert!(text.contains("Work: Implement"));
    assert_eq!(said.criterion().as_str(), "on_plan");
}

#[test]
fn a_session_with_no_plan_and_no_change_is_told_so_rather_than_shown_blanks() {
    let text = brief("", "").question().to_string();
    assert!(text.contains("recorded no plan"), "{text}");
    assert!(text.contains("No file has changed yet."), "{text}");
}

#[test]
fn a_no_objection_reads_as_met() {
    let judged = brief("p", DIFF).read("verdict: met").expect("a pass");
    assert_eq!(judged.verdict, JudgeVerdict::Met);
}

#[test]
fn a_refusal_that_quotes_the_diff_is_placed_in_the_diff_part() {
    let said = brief("p", DIFF);
    let judged = said
        .read(
            "verdict: not_met\n\
             expected: the bound left alone\n\
             produced: \"rows.take(n + 1)\" in the reader\n\
             consequence: one row too many is read",
        )
        .expect("a refusal that cites");
    let cited = judged.cited.expect("asked");
    let at = cited.iter().find(|one| one.region == "diff").expect("placed in the diff");
    assert!(said.question().lines().nth(at.from_line as usize - 1).is_some_and(|line| line.contains("rows.take(n + 1)")));
}

#[test]
fn a_refusal_that_quotes_words_the_brief_never_held_is_unreadable_and_not_a_concern() {
    let judged = brief("p", DIFF).read(
        "verdict: not_met\n\
         expected: the plan, which says \"a sentence from the transcript that the call was never shown at all\"\n\
         produced: something\n\
         consequence: something else",
    );
    assert!(matches!(judged, Err(Unreadable::RefusalQuotesWhatIsNotThere { .. })), "{judged:?}");
}

#[test]
fn a_session_judge_is_asked_for_each_part_of_a_refusal_in_one_short_sentence() {
    let text = brief("Fix the bound (open)", DIFF).question().to_string();
    assert!(text.contains("one short sentence"), "{text}");
    assert!(text.trim_end().ends_with("say only what is missing or wrong."), "{text}");
}
