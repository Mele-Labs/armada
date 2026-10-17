//! The pieces `verification` ships, each editable: a Judge's briefs and the turns the gate gives a Drone. Fleet hands the override in.

use super::prompts::{prompt, PromptSection};
use super::{Entry, Words};

pub const PROMPT_JUDGE_OPENING: Words = Words("prompts.judgeOpening");
pub const PROMPT_JUDGE_QUESTION: Words = Words("prompts.judgeQuestion");
pub const PROMPT_JUDGE_ANSWER: Words = Words("prompts.judgeAnswer");
pub const PROMPT_JUDGE_RISK: Words = Words("prompts.judgeRisk");
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

pub(super) const JUDGE: &[Entry] = &[
    prompt(PromptSection::Judge, PROMPT_JUDGE_OPENING, "What it is doing", "What a Judge is told before anything else.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_QUESTION, "The question", "The line a criterion's question follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_ANSWER, "How it answers", "How a Judge answers. Fleet reads the answer by these field names.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_RISK, "Whether it is risky", "What a panel's members are asked beside a refusal. Fleet reads the answer by the field name risky.", &[]),
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
];
