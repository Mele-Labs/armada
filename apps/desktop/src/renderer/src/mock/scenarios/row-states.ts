// One Job in each of the five states a Cockpit row reads at a glance: needs him (a question, stuck, red Checks with
// the agent idle), working, waiting on something that is not him (a pull request in the merge queue, one on its Checks
// with auto-merge on, a deploy watch beside a pull request on its Checks), idle (and an unread reply), and a draft. The walk `rowStates` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment, SessionChecks } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pr = (number: number, title: string, checks: SessionChecks, more: { auto?: boolean; queued?: true } = {}): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch: `armada/rs-${number}`,
  address: `https://git.example/armada/pull/${number}`,
  checks,
  state: "open",
  auto: more.auto ?? false,
  ...(more.queued === undefined ? {} : { queued: true as const }),
});

const job = (id: string, title: string, says: string, minutes: number, more: Partial<Session>): Session => ({
  id,
  title,
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(minutes),
  rows: [{ id: `${id}-1`, at: "13:00:00", kind: "message", from: { kind: "agent" }, text: says }],
  attachments: [],
  work: { proposal: { workflow: null } },
  ...more,
});

const asking = job("rs-asks", "Pick the retry limit", "Three retries or five?", 3, {
  waitingFor: [{ id: "q-rs", text: "Three retries or five?", since: ago(3), source: "ask_card", options: [{ label: "Three" }, { label: "Five" }] }],
});
const stuck = job("rs-stuck", "Migrate the billing tables", "Running the migration against the copy.", 25, { turn: { state: "working" }, standing: "stuck" });
const red = job("rs-red", "Trim the session cache", "Opened #3079.", 9, { attachments: [pr(3079, "Trim the session cache", { state: "failed", failing: "desktop_unit" })] });
const working = job("rs-working", "Little Workflow State Machines", "Wiring the rule editor to the new route.", 1, { turn: { state: "working" }, standing: "working" });
const queued = job("rs-queued", "Keep for Every Job Pending Mark", "Nothing more to do until #3082 merges or fails.", 6, {
  attachments: [pr(3082, "Show a Keep for every Job waiting on Approve", { state: "passed" }, { auto: true, queued: true })],
});
const checking = job("rs-checking", "Fix Four Filed Issues", "Pushed. Auto-merge is on for #3081.", 4, {
  attachments: [pr(3081, "Fix four filed issues", { state: "pending" }, { auto: true })],
});
const watched = job("rs-watched", "Raise the upload limit", "#408 merged. The Staging deploy is running, and #409 raises the proxy's limit to match.", 12, {
  heldOpenBy: "Staging deploy",
  attachments: [pr(409, "Raise the proxy's body limit", { state: "pending" }, { auto: true })],
});
const idle = job("rs-idle", "Design Pass Spacing", "Slot 16 is up to date with main.", 40, {});
const replied = job("rs-replied", "Walk the gaps", "The three gaps are filed as issues.", 2, { standing: "replied" });
const draft = job("rs-draft", "Demo marketing video", "", 60, { rows: [], start: { asksDelivery: false } });

const base = holding("row-states", "One Job in each of the five row states: needs you, working, waiting on something else, idle and draft");

export const rowStates: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions: () => sessionsStore([asking, stuck, red, working, queued, checking, watched, idle, replied, draft], [], new Map(), []) },
};
