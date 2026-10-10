//! The pieces `verification` ships, each editable: a Judge's briefs and the turns the gate gives a Drone. Fleet hands the override in.

use super::prompts::{prompt, PromptSection};
use super::{Entry, Words};

pub const PROMPT_JUDGE_OPENING: Words = Words("prompts.judgeOpening");
pub const PROMPT_JUDGE_QUESTION: Words = Words("prompts.judgeQuestion");
pub const PROMPT_JUDGE_ANSWER: Words = Words("prompts.judgeAnswer");
pub const PROMPT_JUDGE_REQUEST: Words = Words("prompts.judgeRequest");
pub const PROMPT_JUDGE_REQUEST_DONE_WHEN: Words = Words("prompts.judgeRequestDoneWhen");
pub const PROMPT_JUDGE_READABLE: Words = Words("prompts.judgeReadable");
pub const PROMPT_JUDGE_STANDING: Words = Words("prompts.judgeStanding");
pub const PROMPT_JUDGE_STANDING_UNREADABLE: Words = Words("prompts.judgeStandingUnreadable");
pub const PROMPT_JUDGE_STANDING_CUT: Words = Words("prompts.judgeStandingCut");
pub const PROMPT_JUDGE_CHECKS_RAN: Words = Words("prompts.judgeChecksRan");
pub const PROMPT_JUDGE_PRINTED: Words = Words("prompts.judgePrinted");
pub const PROMPT_JUDGE_PRINTED_TAIL: Words = Words("prompts.judgePrintedTail");
pub const PROMPT_JUDGE_DELIVERABLE: Words = Words("prompts.judgeDeliverable");
pub const PROMPT_JUDGE_PLAN: Words = Words("prompts.judgePlan");
pub const PROMPT_JUDGE_SUMMARY_AFTER: Words = Words("prompts.judgeSummaryAfter");
pub const PROMPT_JUDGE_SUMMARY: Words = Words("prompts.judgeSummary");
pub const PROMPT_JUDGE_ALSO_CHANGED: Words = Words("prompts.judgeAlsoChanged");
pub const PROMPT_JUDGE_DIFF: Words = Words("prompts.judgeDiff");
pub const PROMPT_JUDGE_REFERENCES: Words = Words("prompts.judgeReferences");
pub const PROMPT_JUDGE_GAMING_OPENING: Words = Words("prompts.judgeGamingOpening");
pub const PROMPT_JUDGE_GAMING_BASELINE: Words = Words("prompts.judgeGamingBaseline");
pub const PROMPT_JUDGE_GAMING_NO_BASELINE: Words = Words("prompts.judgeGamingNoBaseline");
pub const PROMPT_JUDGE_GAMING_HOW_TO_READ: Words = Words("prompts.judgeGamingHowToRead");
pub const PROMPT_JUDGE_GAMING_DIFF: Words = Words("prompts.judgeGamingDiff");
pub const PROMPT_JUDGE_GAMING_QUESTION: Words = Words("prompts.judgeGamingQuestion");
pub const PROMPT_JUDGE_GAMING_ANSWER: Words = Words("prompts.judgeGamingAnswer");
pub const PROMPT_JUDGE_PATTERN_ASSERTION_WEAKENED: Words =
    Words("prompts.judgePatternAssertionWeakened");
pub const PROMPT_JUDGE_PATTERN_TEST_SCOPE_NARROWED: Words =
    Words("prompts.judgePatternTestScopeNarrowed");
pub const PROMPT_JUDGE_PATTERN_TAUTOLOGICAL_TEST: Words =
    Words("prompts.judgePatternTautologicalTest");
pub const PROMPT_JUDGE_PATTERN_NO_FINDINGS: Words = Words("prompts.judgePatternNoFindings");
pub const PROMPT_JUDGE_PATTERN_FINDINGS_NOT_TIED: Words =
    Words("prompts.judgePatternFindingsNotTied");
pub const PROMPT_JUDGE_PATTERN_FINDINGS_GENERIC: Words =
    Words("prompts.judgePatternFindingsGeneric");
pub const PROMPT_JUDGE_SECOND_OPENING: Words = Words("prompts.judgeSecondOpening");
pub const PROMPT_JUDGE_SECOND_QUESTION: Words = Words("prompts.judgeSecondQuestion");
pub const PROMPT_JUDGE_SECOND_CITED: Words = Words("prompts.judgeSecondCited");
pub const PROMPT_JUDGE_SECOND_AT_LINE: Words = Words("prompts.judgeSecondAtLine");
pub const PROMPT_JUDGE_SECOND_REMOVED_LINE: Words = Words("prompts.judgeSecondRemovedLine");
pub const PROMPT_JUDGE_SECOND_HOW_TO_WEIGH: Words = Words("prompts.judgeSecondHowToWeigh");
pub const PROMPT_JUDGE_SECOND_ANSWER: Words = Words("prompts.judgeSecondAnswer");
pub const PROMPT_JUDGE_CONVERGE_OPENING: Words = Words("prompts.judgeConvergeOpening");
pub const PROMPT_JUDGE_CONVERGE_DECLARED: Words = Words("prompts.judgeConvergeDeclared");
pub const PROMPT_JUDGE_CONVERGE_OFF_PLAN: Words = Words("prompts.judgeConvergeOffPlan");
pub const PROMPT_JUDGE_CONVERGE_LAST_TIME: Words = Words("prompts.judgeConvergeLastTime");
pub const PROMPT_JUDGE_CONVERGE_LAST_TIME_RULE: Words = Words("prompts.judgeConvergeLastTimeRule");
pub const PROMPT_JUDGE_CONVERGE_DIFF: Words = Words("prompts.judgeConvergeDiff");
pub const PROMPT_JUDGE_CONVERGE_WRITTEN: Words = Words("prompts.judgeConvergeWritten");
pub const PROMPT_JUDGE_CONVERGE_NOTHING_WRITTEN: Words =
    Words("prompts.judgeConvergeNothingWritten");
pub const PROMPT_JUDGE_CONVERGE_TOO_BIG: Words = Words("prompts.judgeConvergeTooBig");
pub const PROMPT_JUDGE_CONVERGE_NOTHING_ELSE: Words = Words("prompts.judgeConvergeNothingElse");
pub const PROMPT_JUDGE_CONVERGE_ALONGSIDE: Words = Words("prompts.judgeConvergeAlongside");
pub const PROMPT_JUDGE_CONVERGE_QUESTION: Words = Words("prompts.judgeConvergeQuestion");
pub const PROMPT_JUDGE_CONVERGE_CITES_DIFF: Words = Words("prompts.judgeConvergeCitesDiff");
pub const PROMPT_JUDGE_CONVERGE_CITES_FILE: Words = Words("prompts.judgeConvergeCitesFile");
pub const PROMPT_JUDGE_CONVERGE_ANSWER: Words = Words("prompts.judgeConvergeAnswer");
pub const PROMPT_JUDGE_WIDEN_OPENING: Words = Words("prompts.judgeWidenOpening");
pub const PROMPT_JUDGE_WIDEN_STEP: Words = Words("prompts.judgeWidenStep");
pub const PROMPT_JUDGE_WIDEN_DELIVERABLE: Words = Words("prompts.judgeWidenDeliverable");
pub const PROMPT_JUDGE_WIDEN_HELD: Words = Words("prompts.judgeWidenHeld");
pub const PROMPT_JUDGE_WIDEN_ASKED: Words = Words("prompts.judgeWidenAsked");
pub const PROMPT_JUDGE_WIDEN_FENCED: Words = Words("prompts.judgeWidenFenced");
pub const PROMPT_JUDGE_WIDEN_WHY: Words = Words("prompts.judgeWidenWhy");
pub const PROMPT_JUDGE_WIDEN_QUESTION: Words = Words("prompts.judgeWidenQuestion");
pub const PROMPT_JUDGE_WIDEN_ANSWER: Words = Words("prompts.judgeWidenAnswer");
pub const PROMPT_OUTCOME_VERIFIED_ALL: Words = Words("prompts.outcomeVerifiedAll");
pub const PROMPT_OUTCOME_VERIFIED_NONE_COVERED: Words = Words("prompts.outcomeVerifiedNoneCovered");
pub const PROMPT_OUTCOME_VERIFIED_SOME: Words = Words("prompts.outcomeVerifiedSome");
pub const PROMPT_OUTCOME_ACCEPTED: Words = Words("prompts.outcomeAccepted");
pub const PROMPT_OUTCOME_GO_ON: Words = Words("prompts.outcomeGoOn");
pub const PROMPT_OUTCOME_LAST: Words = Words("prompts.outcomeLast");
pub const PROMPT_OUTCOME_HANDED_BACK: Words = Words("prompts.outcomeHandedBack");
pub const PROMPT_OUTCOME_WORK_AGAIN: Words = Words("prompts.outcomeWorkAgain");
pub const PROMPT_BASE_UP_TO_DATE: Words = Words("prompts.baseUpToDate");
pub const PROMPT_BASE_CONFLICTED: Words = Words("prompts.baseConflicted");
pub const PROMPT_BASE_NOT_MOVED: Words = Words("prompts.baseNotMoved");

pub(super) const JUDGE: &[Entry] = &[
    prompt(PromptSection::Judge, PROMPT_JUDGE_OPENING, "What it is doing", "What a Judge is told before anything else.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_QUESTION, "The question", "The line a criterion's question follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_ANSWER, "How it answers", "How a Judge answers. Fleet reads the answer by these field names.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_REQUEST, "The request", "The line the request follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_REQUEST_DONE_WHEN, "The Job's criteria", "The line the Job's acceptance criteria follow.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_READABLE, "Reading the checkout", "What a Judge is told about reading the repository.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING, "The repository's rules", "The line the repository's own rules follow.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING_UNREADABLE, "Rules that would not read", "{path} is the file the repository names.", &["path"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING_CUT, "Rules cut to fit", "{path} is the file, {whole} its size and {cap} what a brief carries.", &["path", "whole", "cap"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_CHECKS_RAN, "Checks that ran", "The line the checks that already ran follow.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_PRINTED, "What a check printed", "{check} is the check.", &["check"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_PRINTED_TAIL, "The end of what a check printed", "{check} is the check.", &["check"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_DELIVERABLE, "The file a step wrote", "{file} is where Fleet read it.", &["file"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_PLAN, "The Job's plan", "The line the Job's plan follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_SUMMARY_AFTER, "The summary, after a document", "The line the submitted summary follows where a document is above it.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_SUMMARY, "The summary", "The line the submitted summary follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_ALSO_CHANGED, "Files also changed", "The line the diff follows where a document is above it.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_DIFF, "The diff", "The line the diff follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_REFERENCES, "Earlier steps", "The line what earlier steps established follows.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_OPENING, "What it is doing", "What a gaming look is told before anything else.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_BASELINE, "The earlier step", "{step} names the earlier step.", &["step"]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_NO_BASELINE, "No earlier step", "Where there is no earlier step to measure against.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_HOW_TO_READ, "Reading the diff", "What the diff is and what its markers mean.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_DIFF, "The diff", "The line the diff follows.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_QUESTION, "The question", "The line the pattern's question follows.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_GAMING_ANSWER, "How it answers", "How a gaming look answers. Fleet reads the answer by these field names.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_ASSERTION_WEAKENED, "An assertion weakened", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_TEST_SCOPE_NARROWED, "Tests narrowed", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_TAUTOLOGICAL_TEST, "A test that cannot fail", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_NO_FINDINGS, "A review with no findings", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_FINDINGS_NOT_TIED, "Findings off the diff", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeGaming, PROMPT_JUDGE_PATTERN_FINDINGS_GENERIC, "Generic findings", "The question a gaming look is asked about this pattern.", &[]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_OPENING, "What it is doing", "What the second reader of a flag is told first.", &[]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_QUESTION, "The question", "The line the first reader's question follows.", &[]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_CITED, "What was cited", "The line what the first reader cited follows.", &[]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_AT_LINE, "Where it is", "{file} and {line} say where the cited line is.", &["file", "line"]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_REMOVED_LINE, "A removed line", "{file} is where the removed line was.", &["file"]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_HOW_TO_WEIGH, "How to weigh it", "How the second reader weighs the flag.", &[]),
    prompt(PromptSection::JudgeSecondReading, PROMPT_JUDGE_SECOND_ANSWER, "How it answers", "How the second reader answers. Fleet reads its last two lines.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_OPENING, "What it is doing", "What a convergence look is told first.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_DECLARED, "Declared paths", "The line the declared paths follow.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_OFF_PLAN, "Off the plan", "The line the paths changed outside the plan follow.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_LAST_TIME, "Last time", "The line what failed last time follows.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_LAST_TIME_RULE, "After a failure", "How a look answers past a failed attempt.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_DIFF, "The diff", "The line the diff follows where it is the product.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_WRITTEN, "The file so far", "{file} is the file the step writes.", &["file"]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_NOTHING_WRITTEN, "An empty file", "{file} is the file the step writes.", &["file"]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_TOO_BIG, "A file too big", "{why} says why it is not shown.", &["why"]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_NOTHING_ELSE, "Nothing else changed", "Where nothing beside the file changed.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_ALONGSIDE, "The diff alongside", "The line the diff beside a file follows.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_QUESTION, "The question", "The convergence question.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_CITES_DIFF, "Cite the diff", "What a finding names where the diff is the product.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_CITES_FILE, "Cite the file", "What a finding names where a file is the product.", &[]),
    prompt(PromptSection::JudgeConvergence, PROMPT_JUDGE_CONVERGE_ANSWER, "How it answers", "How a convergence look answers. {cited} is one of the two above.", &["cited"]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_OPENING, "What it is doing", "What a widening look is told first.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_STEP, "The step", "{step} is the step's label.", &["step"]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_DELIVERABLE, "The step's file", "{file} is the file the step writes.", &["file"]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_HELD, "Declared paths", "The line the declared paths follow.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_ASKED, "Asked-for paths", "The line the paths asked for follow.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_FENCED, "Fenced paths", "What the look is told about paths the step was fenced out of.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_WHY, "The asker's reason", "The line the asker's reason follows.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_QUESTION, "The question", "The widening question.", &[]),
    prompt(PromptSection::JudgeWidening, PROMPT_JUDGE_WIDEN_ANSWER, "How it answers", "How a widening look answers. Fleet reads the answer by these field names.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_VERIFIED_ALL, "Outcome: every check passed", "The opening when the gate passed and every declared check ran. {step} is the step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_VERIFIED_NONE_COVERED, "Outcome: no check covered it", "The opening when no declared check covers what changed. {step} is the step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_VERIFIED_SOME, "Outcome: some checks ran", "The opening when only the checks covering what changed ran. {step} is the step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_ACCEPTED, "Outcome: accepted by a person", "The opening when a person accepted the step. {step} is the step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_GO_ON, "Outcome: go on", "What follows the opening when another step comes next. {step} is that step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_LAST, "Outcome: the last part", "What follows the opening when the step was the last.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_HANDED_BACK, "Outcome: handed back", "The line the failed checks follow when the work goes back to its Drone. {step} is the step.", &["step"]),
    prompt(PromptSection::InjectedTurns, PROMPT_OUTCOME_WORK_AGAIN, "Outcome: work it again", "The close of a hand-back.", &[]),
    prompt(PromptSection::InjectedTurns, PROMPT_BASE_UP_TO_DATE, "The base moved: brought up to date", "{base} is the base branch and {commits} how many commits it moved.", &["base", "commits"]),
    prompt(PromptSection::InjectedTurns, PROMPT_BASE_CONFLICTED, "The base moved: conflicts left", "{base} is the base branch and {files} the files with markers.", &["base", "files"]),
    prompt(PromptSection::InjectedTurns, PROMPT_BASE_NOT_MOVED, "The base moved: could not follow", "{base} is the base branch.", &["base"]),
];
