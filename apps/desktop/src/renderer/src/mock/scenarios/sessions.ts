// Sessions, drawn ahead of Fleet: raw conversations with a hosted agent, beside
// Helm. The Board and Cleanup are the real ones over real fixtures; the
// Sessions themselves are draft data (`packages/screens/src/draft/sessions.ts`)
// the scenario's script plays moment by moment. The walk `sessions` plays it.
//
// Jobs 52 and 53 are the two the walk's own Session dispatches. They are on
// Cleanup from the start and join the Board when the dispatch moment comes.

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import type { WorktreeSlot, WorktreesHeld } from "@armada/protocol";
import { escalatedGateFailure, review, running } from "@armada/jobs/fixtures/build/index";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import type { Session } from "@armada/screens/src/draft/sessions";
import { sessionsStore } from "../sessions/script";
import type { BoardControl } from "../sessions/script";

const AT = "/Users/user/armada/.armada/slots";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** The fixture on its own branch, on the row and on the detail. */
function onBranch(fixture: JobFixture, branch: string): JobFixture {
  const { watched } = fixture;
  return {
    ...fixture,
    job: { ...fixture.job, branch },
    watched: watched.state === "read" ? { ...watched, detail: { ...watched.detail, job: { ...watched.detail.job, branch } } } : watched,
  };
}

const plain = onBranch(asRow(review(), 44, "cap-log-reader", "Cap the log reader"), "job/44-cap-log-reader");
const pin = onBranch(asRow(running(), 52, "pin-store-clock", "Pin the store clock in every test"), "fix/52-pin-store-clock");
const sleeps = onBranch(asRow(running(), 53, "retire-sleeps", "Retire sleep calls in the store tests"), "fix/53-retire-sleeps");
// A Job that stopped at its gate, which the owner tags in a Session to find out why.
const stuck = onBranch(asRow(escalatedGateFailure(), 55, "retry-backoff", "Cap the retry backoff"), "fix/55-retry-backoff");
// The Job the Review act dispatches on a pull request: Fleet's proposer reads the link and picks Code Review.
const inspecting = onBranch(asRow(running(), 54, "review-1843", "Code review of #1843"), "review/1843");

const bay = (n: number, branch: string, held: WorktreeSlot["held"], minutes: number): WorktreeSlot => ({
  manifest_id: "armada",
  slot: n,
  path: `${AT}/slot-${n}`,
  base: "main",
  warm: true,
  behind: 0,
  since: ago(minutes),
  branch,
  held,
});

const job = (fixture: JobFixture): WorktreeSlot["held"] => ({ state: "job", job_id: fixture.job.id, job_title: fixture.job.title });

const held = (): WorktreesHeld => ({
  slots: [
    bay(1, plain.job.branch!, job(plain), 90),
    { manifest_id: "armada", slot: 2, path: `${AT}/slot-2`, base: "main", warm: true, behind: 0, held: { state: "free" } },
    bay(3, "fix/flaky-store", { state: "session", holder: "Session s7" }, 6),
    bay(4, pin.job.branch!, job(pin), 4),
    bay(5, "rel/notes-script", { state: "session", holder: "Session s2" }, 40),
    bay(6, sleeps.job.branch!, job(sleeps), 4),
    bay(7, "spike/store-migrations", { state: "session", holder: "Session s3" }, 120),
    bay(8, inspecting.job.branch!, job(inspecting), 1),
    bay(9, stuck.job.branch!, job(stuck), 55),
  ],
  worktrees: [],
});

const base = holding("sessions", "Sessions beside Helm: a blank one, its first write, and what it accumulates", [plain, stuck]);
const dispatched = (fixture: JobFixture, number: number, slot: number) => ({
  id: fixture.job.id,
  number,
  title: fixture.job.title,
  branch: fixture.job.branch!,
  slot,
});

export const s200Sessions: Scenario = {
  ...base,
  reads: { ...base.reads, [stuck.job.id]: stuck, [pin.job.id]: pin, [sleeps.job.id]: sleeps, [inspecting.job.id]: inspecting },
  held: held(),
  // A request Bridge sends is a Job on the Board, as it is on a real Fleet; here, the review Job.
  behaves: (fleet) => ({
    proposeFromRequest: () => {
      fleet.publish({ jobs: [...fleet.state().jobs, inspecting.job] });
      // The call is still out, so nothing waits on an answer.
      return new Promise(() => {});
    },
  }),
  draft: { sessions: (board) => sessionsOf(board) },
};

/** The walk's Sessions store, with `more` Sessions open beside the usual ones. */
export function sessionsOf(board: BoardControl, more: readonly Session[] = []) {
  return sessionsStore([dispatched(pin, 52, 4), dispatched(sleeps, 53, 6)], dispatched(inspecting, 54, 8), [{ ...dispatched(plain, 44, 1), state: "review" }, { ...dispatched(stuck, 55, 9), state: "escalated" }], board, [pin.job, sleeps.job], more);
}
