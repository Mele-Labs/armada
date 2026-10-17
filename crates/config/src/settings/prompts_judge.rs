//! The pieces `verification` ships, each editable: a Judge's briefs and the turns the gate gives a Drone. Fleet hands the override in.

use super::prompts::{prompt, PromptSection};
use super::{Entry, Words};

pub const PROMPT_JUDGE_OPENING: Words = Words("prompts.judgeOpening");
pub const PROMPT_JUDGE_QUESTION: Words = Words("prompts.judgeQuestion");
pub const PROMPT_JUDGE_ANSWER: Words = Words("prompts.judgeAnswer");
pub const PROMPT_JUDGE_RISK: Words = Words("prompts.judgeRisk");
pub const PROMPT_JUDGE_READABLE: Words = Words("prompts.judgeReadable");
pub const PROMPT_JUDGE_STANDING: Words = Words("prompts.judgeStanding");
pub const PROMPT_JUDGE_STANDING_UNREADABLE: Words = Words("prompts.judgeStandingUnreadable");
pub const PROMPT_JUDGE_STANDING_CUT: Words = Words("prompts.judgeStandingCut");

pub(super) const JUDGE: &[Entry] = &[
    prompt(PromptSection::Judge, PROMPT_JUDGE_OPENING, "What it is doing", "What a Judge is told before anything else.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_QUESTION, "The question", "The line a criterion's question follows.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_ANSWER, "How it answers", "How a Judge answers. Fleet reads the answer by these field names.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_RISK, "Whether it is risky", "What a panel's members are asked beside a refusal. Fleet reads the answer by the field name risky.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_READABLE, "Reading the checkout", "What a Judge is told about reading the repository.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING, "The repository's rules", "The line the repository's own rules follow.", &[]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING_UNREADABLE, "Rules that would not read", "{path} is the file the repository names.", &["path"]),
    prompt(PromptSection::Judge, PROMPT_JUDGE_STANDING_CUT, "Rules cut to fit", "{path} is the file, {whole} its size and {cap} what a brief carries.", &["path", "whole", "cap"]),
];
