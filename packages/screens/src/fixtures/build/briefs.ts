// Kept briefs as `get_brief` answers them: the text Fleet wrote before a Judge
// or gaming call went out, laid in `verification`'s own order — the request,
// the evidence, the diff, then the question and the answer format last.
//
// The wording of the fixed parts is copied from `crates/verification`
// (`judge.rs`, `gaming.rs`) so the panel is drawn against what a real brief
// says, not a paraphrase of it.

import type { BriefRead } from "@armada/protocol";

/**
 * What a Judge brief is about: the step, the request, the evidence, the diff
 * and the question. No `diff` is a step whose product is a note, the plan's.
 */
export type JudgeBriefOf = {
  step: string;
  request: string;
  said: string;
  evidence: string[];
  diff?: string[];
  question: string;
};

/** A Judge's brief for one criterion, as Fleet keeps it. */
export function judgeBrief(path: string, of: JudgeBriefOf): BriefRead {
  return kept(path, [
    "You are verifying one condition on work somebody else did. Answer only the question at the end.",
    "",
    `Step: ${of.step}`,
    "",
    "What was asked for, in the requester's own words. This is the request the whole Job answers, and it is the standard the work below is measured against rather than something under judgment:",
    "",
    `  the request: ${of.request}`,
    `  what they said about it: ${of.said}`,
    "",
    ...of.evidence,
    "",
    ...(of.diff === undefined ? [] : ["The change, as a diff:", "", ...of.diff, ""]),
    "The question, which is yes or no:",
    "",
    of.question,
    "",
    "Answer with nothing but the lines below.",
    "",
    "If the evidence satisfies the criterion:",
    "",
    "    verdict: met",
    "",
    "If it does not:",
    "",
    "    verdict: not_met",
    "    expected: <what should be seen if the work were right>",
    "    produced: <what is seen instead>",
    "    consequence: <what that difference does to whoever consumes it>",
    "",
    "Each of the three is one line and names something in the work above. A refusal that could be written about any other piece of work is not a refusal.",
  ]);
}

/** What a gaming brief is about: the step, the earlier step's yardstick, the diff and the question. */
export type GamingBriefOf = {
  step: string;
  earlier: string;
  established: string;
  diff: string[];
  question: string;
};

/** A gaming check's brief for one pattern, as Fleet keeps it. */
export function gamingBrief(path: string, of: GamingBriefOf): BriefRead {
  return kept(path, [
    "You are checking whether a change was made to look finished rather than to be finished. Answer only the question at the end.",
    "",
    `Step: ${of.step}`,
    "",
    `What the earlier step \`${of.earlier}\` established, which this change is measured against:`,
    "",
    `  it now does: ${of.established}`,
    "",
    "The whole change is below as one diff, with nothing of it left out — so anywhere in this diff is still inside this change, and something removed in one place may be done again in another.",
    "",
    "A line marked `+` is what this change writes. A line marked `-` is what it removes. A line with neither marker is a header or unchanged context: code that was already there and that this change leaves exactly as it found it.",
    "",
    "The change, as a diff:",
    "",
    ...of.diff,
    "",
    "The question, which is yes or no:",
    "",
    of.question,
    "",
    "Answer with nothing but the lines below.",
    "",
    "If the change shows no sign of it:",
    "",
    "    flag: no",
    "",
    "If it does:",
    "",
    "    flag: yes",
    "    cited: <the file, line or assertion this is about>",
    "",
    "`cited` names something in the diff above. A flag that could be written about any other change is not a flag.",
  ]);
}

/** A whole brief: short enough that the window is all of it. */
function kept(path: string, lines: string[]): BriefRead {
  return {
    ok: true,
    brief: {
      path,
      lines,
      from_line: 1,
      total_lines: lines.length,
      bytes: lines.reduce((sum, line) => sum + new TextEncoder().encode(line).length + 1, 0),
      whole: true,
    },
  };
}

/** What a kept brief weighs, so the row Fleet lists and the panel agree. */
export function briefBytes(read: BriefRead | undefined): number | undefined {
  return read?.ok === true ? read.brief.bytes : undefined;
}

/** A brief's file name, the key `get_brief` and a fixture's `briefs` use. */
export function briefName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}
