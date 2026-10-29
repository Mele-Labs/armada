// The Jobs `teach-anything` opens, one in each of three repositories. The first holds what Teach is pressed on: a Jira key in a
// message, a command, a Fleet notice, a red Check and a plan task. The second and third hold a key of their own, in repositories
// whose Jira sites differ: one is found in a pull request, one is asked.

import type { SessionPlan, SessionRow, SessionRecord } from "@armada/protocol";

import { held, hosted } from "../sessions-fleet";

export const ARMADA_JOB = "01TEACHARMADAAAAAAAAAAAAAA";
export const PAYMENTS_JOB = "01TEACHPAYMENTSBBBBBBBBBBB";
export const LEDGER_JOB = "01TEACHLEDGERCCCCCCCCCCCCC";

/** The question Fleet puts to the person when it finds no Jira site for the third repository. */
export const SITE_ASK = "jira-site:ledger";

export const ARMADA_TITLE = "Pin the clock in the store";
export const PAYMENTS_TITLE = "Round the payout totals";
export const LEDGER_TITLE = "Export the month in CSV";

const AT = "2026-10-22T09:00:00.000Z";
const at = (minute: number) => `2026-10-22T09:${String(minute).padStart(2, "0")}:00.000Z`;
const FLEET = { kind: "session", id: "fleet", title: "Fleet" } as const;

/** The message a Jira key is taught from. */
export const KEY_MESSAGE = "I am going to create a PR for STORE-1234 and then I will monitor CI for pass/fail.";
export const PAYMENTS_MESSAGE = "The totals are rounded per line. PAY-88 asks for it per invoice, so I am changing both.";
export const LEDGER_MESSAGE = "The export reads the month's entries once. LED-210 wants the same file split by account.";

export const INSTALL = "Bash npm install";
export const BEHIND = "slot-14 is 25 commits behind main. Merge main into your branch before your next push.";
export const CHECKS_FAILED = "#2712 failed its checks at 3f9a1c2. desktop_unit: 2 tests failed.";
export const CLASH = "main moved under your branch: 3 files overlap with your changes.";
export const TASK = "Build the clock panel";

/** The notices Fleet sends after the batch is taught: two more slots behind main. */
export const LATER_NOTICES = [
  { text: "slot-9 is 12 commits behind main. Merge main into your branch before your next push.", minute: 50 },
  { text: "slot-3 is 7 commits behind main. Merge main into your branch before your next push.", minute: 55 },
] as const;
export const BEHIND_AT = at(40);

const PLAN: SessionPlan = {
  version: 1,
  summary: "",
  recorded_at: AT,
  approved: true,
  approved_at: AT,
  scope: ["crates/store", "apps/desktop"],
  groups: [
    {
      id: "g1",
      agent: "bridge-engineer",
      title: "Show the clock",
      paths: ["apps/desktop"],
      lands_in: { label: "PR 1" },
      tasks: [
        { id: "t1", text: "Pin the clock in the store tests", state: "working" },
        { id: "t2", text: TASK, state: "open" },
      ],
    },
  ],
};

export const armadaRecord = (): SessionRecord =>
  hosted(ARMADA_JOB, { title: ARMADA_TITLE, attachments: [held("slot", "14"), held("branch", "armada/session-84ed9507")], plan: PLAN });

export const paymentsRecord = (): SessionRecord =>
  hosted(PAYMENTS_JOB, { title: PAYMENTS_TITLE, manifest_id: "payments", cwd: "/Users/user/code/payments", attachments: [{ ...held("slot", "2"), manifest_id: "payments" }] });

export const ledgerRecord = (): SessionRecord =>
  hosted(LEDGER_JOB, { title: LEDGER_TITLE, manifest_id: "ledger", cwd: "/Users/user/code/ledger", attachments: [{ ...held("slot", "5"), manifest_id: "ledger" }] });

export const teachSessions = (): SessionRecord[] => [armadaRecord(), paymentsRecord(), ledgerRecord()];

export const teachThreads = (): Record<string, SessionRow[]> => ({
  [ARMADA_JOB]: [
    { kind: "message", id: "a1", at: at(0), from: { kind: "you" }, text: "Fix the flaky store test and get it reviewed." },
    { kind: "tool", id: "a2", at: at(1), text: INSTALL },
    { kind: "tool", id: "a3", at: at(2), text: "Bash npm test -- store" },
    { kind: "message", id: "a4", at: at(10), from: { kind: "agent" }, text: KEY_MESSAGE },
    { kind: "check", id: "a5", at: at(20), name: "desktop_unit", run: 41, state: "failed" },
    { kind: "message", id: "n1", at: at(30), from: FLEET, text: CHECKS_FAILED },
    { kind: "message", id: "a6", at: at(32), from: { kind: "agent" }, text: "Two of the store tests failed on a timer. I am reading them." },
    { kind: "message", id: "n2", at: at(35), from: FLEET, text: CLASH },
    { kind: "message", id: "n3", at: BEHIND_AT, from: FLEET, text: BEHIND, notice: "behind_base" },
    { kind: "message", id: "a7", at: at(42), from: { kind: "agent" }, text: "The branch is behind, and none of that touches the store." },
  ],
  [PAYMENTS_JOB]: [
    { kind: "message", id: "p1", at: at(0), from: { kind: "you" }, text: "Round payouts once per invoice." },
    { kind: "message", id: "p2", at: at(5), from: { kind: "agent" }, text: PAYMENTS_MESSAGE },
  ],
  [LEDGER_JOB]: [
    { kind: "message", id: "l1", at: at(0), from: { kind: "you" }, text: "Export the month's entries in CSV." },
    { kind: "message", id: "l2", at: at(5), from: { kind: "agent" }, text: LEDGER_MESSAGE },
  ],
});
