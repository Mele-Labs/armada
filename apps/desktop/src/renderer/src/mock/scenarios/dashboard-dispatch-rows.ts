// The Dashboard under way, as tiles: a dozen Jobs and Sessions running in every state (a Drone at
// work, a Check running, a Judge deciding, one waiting on the owner, one waiting on a resource, a
// one-step Job, two queued, Sessions working, idle and asking), and in Done the Jobs and Sessions
// that are over, with the merge line's landed branches, which Done does not list. The walk
// `dashboard-dispatch-rows` plays it.

import { repository } from "@armada/screens/src/fixtures/build/base";
import { mergeLines } from "@armada/screens/src/fixtures/build/merge-line";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import type { NowView } from "@armada/jobs/draft/now";
import { completedFailed, completedSuccess, queued } from "@armada/jobs/fixtures/build/index";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;

/** A Job on the picked repository, not yet cleared, so it stays on the Dashboard. */
function kept(fixture: JobFixture): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, owner_manifest_id: OWNER };
  return { ...fixture, job };
}

/** The Job with its workflow cut to its first step, so its tile has no pips to draw. */
function oneStep(fixture: JobFixture): JobFixture {
  const [workflow] = fixture.workflows;
  const step = workflow!.steps[0]!;
  return { ...fixture, workflows: [{ ...workflow!, id: "one-step", steps: [step] }], job: { ...fixture.job, workflow_id: "one-step", current_step_id: step.step_id } };
}

const run = (at: number, slug: string, title: string) => kept(asRow(featureRunning(), at, slug, title));

const pin = run(81, "dash-pin", "Pin the store clock");
const fold = run(82, "dash-fold", "Fold the two notification routes into one and drop the retired route with the tests that covered it");
const cap = run(83, "dash-cap", "Cap the retry backoff");
const split = run(84, "dash-split", "Split the writer from the clock");
const index = run(85, "dash-index", "Index the record files");
const rename = kept(oneStep(asRow(featureRunning(), 86, "dash-rename", "Rename the gate verbs")));
const retire = kept(asRow(queued(), 87, "dash-retire", "Retire the sleep calls in the store tests"));
const move = kept(asRow(queued(), 88, "dash-move", "Move the store reads"));
const shipped = kept(asRow(completedSuccess(), 89, "dash-shipped", "Add the heartbeat clock to the writer"));
const shipped2 = kept(asRow(completedSuccess(), 90, "dash-shipped-2", "Cache the manifest read"));
const failed = kept(asRow(completedFailed(), 91, "dash-failed", "Shorten the reconnect wait"));

const STEP = { id: "implement", name: "Implement" } as const;
const DRONE = { key: "d", of: "drone", name: "Implement Drone", line: "Edit crates/store/src/clock.rs", step: STEP, tail: ["Read crates/store/src/clock.rs", "Edit crates/store/src/clock.rs"], state: "running" } as const;
const CHECK_ACTS = [
  { key: "retry", glyph: "retry", said: "Retry now" },
  { key: "skip", glyph: "skip", said: "Skip check" },
] as const;

const now: Record<string, NowView> = {
  [pin.job.id]: { running: [DRONE] },
  [fold.job.id]: {
    running: [
      { key: "c1", of: "check", name: "typecheck", step: STEP, state: "passed" },
      { key: "c2", of: "check", name: "store", step: STEP, tail: ["running 212 tests", "test writer::flushes_on_close ..."], state: "running", acts: CHECK_ACTS },
    ],
  },
  [cap.job.id]: {
    running: [{ key: "j", of: "judge", name: "Judge on Implement", step: STEP, tail: ["Reading the diff against the criteria", "Criterion 2: met"], state: "running" }],
  },
  [split.job.id]: {
    asks: [
      {
        key: "p",
        kind: "plan",
        decisions: [
          {
            id: "shape",
            question: "Split the clock out of the writer, or wrap it in place?",
            options: [
              { id: "split", label: "Split it out" },
              { id: "wrap", label: "Wrap it in place" },
            ],
          },
        ],
      },
    ],
    running: [DRONE],
  },
  [index.job.id]: { waiting: [{ key: "r", kind: "resource", text: "Worktree slot", step: STEP }] },
  [rename.job.id]: { running: [{ ...DRONE, key: "d2", name: "Plan Drone", line: "Reading the gate verbs" }] },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;

const message = (id: string, from: "agent" | "you", text: string): SessionRow => ({ id, at: "14:00:00", kind: "message", from: { kind: from }, text });
const tool = (id: string, text: string): SessionRow => ({ id, at: "14:00:00", kind: "tool", text });

/** A thread longer than its pane, so the newest message is below the fold until the pane opens at it. */
const THREAD: SessionRow[] = [
  message("m0", "you", "List the migrations since 0042 and say which touch the clock column."),
  tool("m1", "Read crates/store/migrations/0043_rename_clock.sql"),
  tool("m2", "Read crates/store/migrations/0044_add_heartbeat.sql"),
  message("m3", "agent", "Two migrations since 0042: 0043 renames the clock column and 0044 adds the heartbeat table."),
  tool("m4", "Read crates/store/src/schema.rs"),
  tool("m5", "grep -rn clock_at crates/store"),
  message("m6", "agent", "Nine call sites still read clock_at, and 0043 renamed it to written_at."),
  tool("m7", "Read crates/store/src/clock.rs"),
  message("m8", "you", "Which of the nine are in tests?"),
  tool("m9", "grep -rn clock_at crates/store/tests"),
  message("m10", "agent", "Four of the nine, all in crates/store/tests/flaky.rs."),
  tool("m11", "Edit crates/store/tests/flaky.rs"),
  tool("m12", "Edit crates/store/src/clock.rs"),
  message("m13", "agent", "Renamed the four test reads to written_at. Checking the other five call sites."),
  tool("m14", "Read crates/store/src/write.rs"),
  tool("m15", "Read crates/store/src/open.rs"),
  message("m16", "agent", "Reading the migrations against the store schema."),
];

const SESSIONS: Session[] = [
  { id: "s20", title: "Migration notes", turn: { state: "working" }, lastTurn: "14:05", lastTurnAt: ago(1), rows: THREAD, attachments: [] },
  {
    id: "s21",
    title: "Release script",
    turn: quiet,
    lastTurn: "13:40",
    lastTurnAt: ago(25),
    rows: [message("r0", "agent", "The release script now tags from the merge commit rather than the branch tip."), tool("r1", "Edit scripts/release"), message("r2", "agent", "It refuses to run while main is red.")],
    attachments: [],
  },
  {
    id: "s23",
    title: "Flaky store test",
    turn: quiet,
    lastTurn: "14:02",
    lastTurnAt: ago(3),
    rows: [message("f0", "agent", "The flaky test is the 200ms sleep in store_test.rs."), tool("f1", "Edit crates/store/tests/store_test.rs"), message("f2", "agent", "The branch is ready to push.")],
    attachments: [],
    asked: { command: "git push --force-with-lease origin fix/flaky-store" },
  },
  { id: "s24", title: "Lint sweep", turn: { state: "working" }, lastTurn: "14:06", lastTurnAt: ago(2), rows: [tool("l0", "cargo clippy --workspace"), tool("l1", "Edit crates/fleet/src/lib.rs"), tool("l2", "cargo clippy --workspace"), message("l3", "agent", "Eleven warnings left."), tool("l4", "Edit crates/store/src/open.rs")], attachments: [] },
  { id: "s22", title: "Migration spike", dead: "ended", turn: quiet, lastTurn: "11:40", lastTurnAt: ago(150), rows: [message("e0", "agent", "Done."), tool("e1", "Read crates/store/src/open.rs"), message("e2", "agent", "Named migrations apply in order.")], attachments: [] },
  { id: "s25", title: "Release notes", dead: "ended", turn: quiet, lastTurn: "09:10", lastTurnAt: ago(300), rows: [message("n0", "agent", "Done.")], attachments: [] },
];

function build(): Scenario {
  const base = holding("dashboard-dispatch-rows", "The Dashboard as tiles: Jobs and Sessions under way, and what is over", [pin, fold, cap, split, index, rename, retire, move, shipped, shipped2, failed]);
  const none = { id: "x", number: 0, title: "", branch: "", slot: 0 };
  return {
    ...base,
    state: { ...base.state, repository: repository().root, mergeLines: { lines: [mergeLines().lines[0]!] } },
    draft: {
      now,
      sessions: (board) =>
        sessionsStore([none, none], none, [], board, [], SESSIONS, [
          ["s20", "0043 also drops the index on clock_at, so open.rs needs the new one before the first write."],
          ["s20", "Adding the index to open.rs and running the store tests."],
        ]),
    },
  };
}

export const s205DashboardDispatchRows: Scenario = build();
