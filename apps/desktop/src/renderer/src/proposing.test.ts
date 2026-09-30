// Which proposal a window shows, and what a row on it says — the two decisions
// in `proposing.ts` that need no window.

import { expect, test } from "vitest";
import type { JobSummary, ProposalInFlight } from "@armada/protocol";
import type { ProposalAnswer } from "@armada/components";

import { asTheBoardSays, showing } from "./proposing";
import type { ProposalRun } from "./proposing";

const OUT: ProposalInFlight = {
  proposal_id: "01M2D3ZF41001PROPOSAL001",
  client_ref: "bridge-1",
  model: "sonnet",
  since: "2026-09-30T09:00:00Z",
  budget_ms: 600_000,
  reached: "thinking",
};

const MINE: ProposalRun = { sent: "Stop the board flickering", at: { at: "reading" } };

test("nothing out and nothing pressed is no screen", () => {
  expect(showing(null, null, null)).toBeNull();
});

test("a press of its own is what is shown, request and all", () => {
  expect(showing(MINE, OUT, null)).toEqual(MINE);
});

/**
 * A window reopened while its own proposer is out. **The call is known and what
 * it was about is not**, so the run carries no request rather than a sentence
 * standing in for one.
 */
test("a published call this window has no run for is adopted, with no request", () => {
  expect(showing(null, OUT, null)).toEqual({ sent: null, at: { at: "reading" } });
});

/**
 * **Leaving is leaving.** Without this the rail would not work while a proposer
 * is out: the press clears the run, and the next render would adopt the same
 * call straight back onto the screen somebody just left.
 */
test("a call already left behind is not adopted again", () => {
  expect(showing(null, OUT, OUT.proposal_id)).toBeNull();
});

/** And the next call is a different call, so it opens the screen. */
test("a second call after one was left behind is adopted", () => {
  const next = { ...OUT, proposal_id: "01M2D3ZF41001PROPOSAL002" };
  expect(showing(null, next, OUT.proposal_id)).toEqual({ sent: null, at: { at: "reading" } });
});

/** An answer that came back outlives the call being published, and is kept. */
test("what came back is shown even with nothing out", () => {
  const answered: ProposalRun = { sent: "x", at: { at: "proposed", jobs: [] } };
  expect(showing(answered, null, null)).toEqual(answered);
});

const row = (id: string, status: string): JobSummary => ({
  id,
  handle: `handle-${id}`,
  title: `Job ${id}`,
  status,
  workflow_id: "feature",
  owner_manifest_id: "armada",
  origin: "manual",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-09-30T09:00:00Z",
});

const proposed = (status: string): ProposalAnswer => ({
  at: "proposed",
  jobs: [{ id: "job_a", title: "Job job_a", workflow: "feature", status }],
});

/**
 * A Job's status is Fleet's, and approving one changes it. **The row is drawn
 * against the board rather than against the answer** — a proposal held on this
 * screen would otherwise go on saying `needs approval` under a Job already
 * queued, and offer a gate that is no longer anybody's to open.
 */
test("a row follows the board, not the answer", () => {
  const shown = asTheBoardSays(proposed("awaiting_approval"), [row("job_a", "queued")]);
  expect(shown).toEqual(proposed("queued"));
});

test("a row the board holds nothing for keeps what came back", () => {
  const at = proposed("awaiting_approval");
  expect(asTheBoardSays(at, [])).toEqual(at);
});

/** Every other state is not a list of Jobs, so there is nothing to read across. */
test("a wait is handed back untouched", () => {
  const at: ProposalAnswer = { at: "reading" };
  expect(asTheBoardSays(at, [row("job_a", "queued")])).toBe(at);
});
