// Pause and Resume over three Jobs, each holding a bay: one running with files
// nobody committed, and two parked at their review gate. The walk `pausingAJob`
// pauses the running one and the first gate Job from the Board, presses Approve
// on the paused one to meet the Resume confirm, and pauses the second from Cleanup.

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import type { JobSummary, WorktreeHeld, WorktreesHeld, WorktreeSlot } from "@armada/protocol";
import { review, running } from "@armada/screens/src/fixtures/build/index";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";

const AT = "/Users/user/armada/.armada/slots";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The fixture on its own branch, on the row and on the detail: every `build/` fixture is one Job. */
function onBranch(fixture: JobFixture): JobFixture {
  const branch = `armada/${fixture.job.handle}`;
  const { watched } = fixture;
  return {
    ...fixture,
    job: { ...fixture.job, branch },
    watched: watched.state === "read" ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, branch } } } : watched,
  };
}

const working = onBranch(asRow(running(), 71, "debounce", "Debounce the Job Board"));
const gateA = onBranch(asRow(review(), 72, "retry", "Retry a short read once"));
const gateB = onBranch(asRow(review(), 73, "selectors", "Port the settings selectors"));

const bay = (n: number, job: JobSummary): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `${AT}/slot-${n}`,
  base: "main",
  warm: true,
  behind: 0,
  since: ago(30 * n),
  branch: job.branch!,
  held: { state: "job", job_id: job.id, job_title: job.title },
});

const tree = (n: number, job: JobSummary, rest: Partial<WorktreeHeld>): WorktreeHeld => ({
  job_id: job.id,
  job_title: job.title,
  status: job.status,
  last_moved_at: ago(5),
  path: `${AT}/slot-${n}`,
  branch: job.branch!,
  held: [],
  on_disk: true,
  ...rest,
});

const held = (): WorktreesHeld => ({
  slots: [
    bay(1, working.job),
    bay(2, gateA.job),
    bay(3, gateB.job),
    { manifest_id: "armada", slot: 4, path: `${AT}/slot-4`, base: "main", warm: true, behind: 0, held: { state: "free" } },
  ],
  worktrees: [
    tree(1, working.job, {
      held: [
        { why: "uncommitted", files: ["src/reader/debounce.rs", "notes/debounce.md"] },
        { why: "not_terminal", status: "running" },
      ],
    }),
    tree(2, gateA.job, { held: [{ why: "not_terminal", status: "awaiting_review" }] }),
    tree(3, gateB.job, { held: [{ why: "not_terminal", status: "awaiting_review" }] }),
  ],
});

const base = holding("pausing/jobs", "Pause and Resume: a running Job and two at their review gate, each holding a bay", [working, gateA, gateB]);

export const s051PausingAJob: Scenario = { ...base, held: held() };
