// The Job `teach-rule` opens: one in the first repository, holding a pull request, which is where Teach is pressed to make a rule.

import type { SessionRow, SessionRecord } from "@armada/protocol";

import { held, hosted, pullRequest } from "../sessions-fleet";

export const RULE_JOB = "01TEACHRULEAAAAAAAAAAAAAAA";
export const RULE_TITLE = "Pin the clock in the store";
export const RULE_PULL = 2712;
export const RULE_PULL_TITLE = "Pin the store clock in tests";

const at = (minute: number) => `2026-10-30T09:${String(minute).padStart(2, "0")}:00.000Z`;

export const ruleRecord = (): SessionRecord =>
  hosted(RULE_JOB, {
    title: RULE_TITLE,
    attachments: [held("slot", "14"), held("branch", "armada/pin-the-clock"), pullRequest(RULE_PULL, { title: RULE_PULL_TITLE, branch: "armada/pin-the-clock", checks: "passed" })],
  });

export const ruleThreads = (): Record<string, SessionRow[]> => ({
  [RULE_JOB]: [
    { kind: "message", id: "r1", at: at(0), from: { kind: "you" }, text: "Pin the clock in the store tests and open a pull request." },
    { kind: "message", id: "r2", at: at(10), from: { kind: "agent" }, text: `The tests no longer read the wall clock. Opened #${RULE_PULL}.` },
  ],
});
