// `running`, with every Drone it has had as Fleet lists them (protocol 21.3)
// and every row stamped with the Drone whose transcript it is in (21.4).
//
// **Four Drones over three steps, one ended by hand.** Root cause's first Drone
// was killed a minute in and the step run again, so the step carries two
// attempts and the list two Drones on it. Fix's Drone is still running and has
// finished one invocation, so it carries turns and a cost as of that line —
// then a person redirected it, and it is thinking.
//
// Shaped as `crates/api/src/tests/shapes.rs`' `job_drones` and the frame
// `crates/ipc/src/tests/turns.rs` sends: turns and cost omitted where unknown,
// never nought; `drone_id` on every Drone's row.

import type { JobDrones, Turn } from "@armada/protocol";

import type { JobFixture } from "@armada/screens/src/fixtures/fixture";
import { answered, called, DRONE_ID, droneEnded, instructed, JOB_ID, observedWatching, said } from "@armada/screens/src/fixtures/build/base";
import { running } from "@armada/screens/src/fixtures/build/running";

const REPRO_DRONE = "01M1HHJ6XB001BZJZ4BE2RPR0A";
const KILLED_DRONE = "01M1HHJ6XB001BZJZ4BE2K1D0B";
const ROOT_DRONE = "01M1HHJ6XB001BZJZ4BE2R00TC";

/** When Root cause's first Drone was killed, and its second spawned. */
const KILLED_AT = "2026-09-10T14:13:40Z";
const AGAIN_AT = "2026-09-10T14:13:52Z";

export const DRONES_HAD: JobDrones = {
  job_id: JOB_ID,
  drones: [
    {
      drone_id: REPRO_DRONE,
      step_id: "repro",
      state: "done",
      since: "2026-09-10T14:11:15Z",
      ended_at: "2026-09-10T14:12:27Z",
      turns: 4,
      cost_micros: 120_000,
    },
    {
      drone_id: KILLED_DRONE,
      step_id: "root_cause",
      state: "killed",
      since: "2026-09-10T14:12:27Z",
      ended_at: KILLED_AT,
      turns: 2,
      cost_micros: 58_310,
    },
    {
      drone_id: ROOT_DRONE,
      step_id: "root_cause",
      state: "done",
      since: AGAIN_AT,
      ended_at: "2026-09-10T14:16:07Z",
      turns: 9,
      cost_micros: 300_000,
    },
    { drone_id: DRONE_ID, step_id: "fix", state: "running", since: "2026-09-10T14:16:07Z", turns: 6, cost_micros: 95_420 },
  ],
};

// Thinking rows take their own `seq` range, clear of `base.ts`'s counter.
let thinkingSeq = 810_000;

/**
 * One model call's thinking as Fleet sends it: a `thinking` row per estimate,
 * each the call's running total, and — once the call is over — the reasoning
 * block as the one `unrecognised` row `crates/adapters/src/transcript.rs`
 * names it. Fixed figures, so a reading of the run's total is one number.
 */
function thinking(step: string, ts: string, estimates: number[], done = true): Turn[] {
  const rows = estimates.map(
    (estimated_tokens): Turn => ({
      ts,
      seq: (thinkingSeq += 1),
      step,
      by: "drone",
      saw: { event: "thinking", estimated_tokens },
    }),
  );
  const reasoned: Turn = {
    ts,
    seq: (thinkingSeq += 1),
    step,
    by: "drone",
    saw: { event: "unrecognised", kind: "the Drone's reasoning, not carried" },
  };
  return done ? [...rows, reasoned] : rows;
}

/** `rows`, stamped as Fleet stamps a transcript's rows. */
const of = (drone: string, rows: Turn[]): Turn[] => rows.map((row) => ({ ...row, drone_id: drone }));

export function everyDroneHad(): JobFixture {
  const was = running();
  if (was.watched.state !== "read" || was.observed.state !== "watching") return was;
  const rows = was.observed.turns.rows;
  // `running`'s own rows, by step: Reproduction's and Fix's up to its last edit.
  const repro = rows.filter((row) => row.step === "repro");
  const fix = rows.filter((row) => row.step === "fix" && !(row.saw.event === "said" && row.saw.text === "thinking"));
  const redirect: Turn = {
    ts: "2026-09-10T14:22:40Z",
    seq: 800_001,
    step: "fix",
    by: "armada",
    saw: { event: "instructed", occasion: "redirect", text: "Keep the old export so nothing else has to move." },
  };
  const turns: Turn[] = [
    ...of(REPRO_DRONE, repro),
    ...of(KILLED_DRONE, [
      instructed("root_cause", "2026-09-10T14:12:27Z", 2, "the root cause is written down", "Root cause"),
      said("root_cause", "2026-09-10T14:13:00Z", "Reading every reducer in the package before the selectors."),
      called("root_cause", "2026-09-10T14:13:20Z", "call_read_k1", "Read", "packages/settings/src/reducer.ts"),
    ]),
    ...of(ROOT_DRONE, [
      instructed("root_cause", AGAIN_AT, 2, "the root cause is written down", "Root cause"),
      ...thinking("root_cause", "2026-09-10T14:14:05Z", [310, 720]),
      said("root_cause", "2026-09-10T14:14:30Z", "Reading the reducer to find where the memoised selector is defined."),
      droneEnded("root_cause", "2026-09-10T14:16:07Z", 9, 300_000),
    ]),
    ...of(DRONE_ID, [
      ...fix,
      droneEnded("fix", "2026-09-10T14:22:30Z", 6, 95_420),
      redirect,
      ...thinking("fix", "2026-09-10T14:22:55Z", [240, 610, 980]),
      said("fix", "2026-09-10T14:23:10Z", "Re-exporting the selectors from the reducer module as well."),
      called("fix", "2026-09-10T14:23:30Z", "call_edit_3", "Edit", "packages/settings/src/reducer.ts +2 -0"),
      answered("fix", "2026-09-10T14:23:31Z", "call_edit_3"),
      // Mid-thought: the call's estimate is still rising and its block has not arrived.
      ...thinking("fix", "2026-09-10T14:24:00Z", [180, 520, 860, 1140], false),
    ]),
  ];
  const detail = was.watched.detail;
  const steps = detail.steps.map((step) =>
    step.step_id !== "root_cause"
      ? step
      : {
          ...step,
          attempts: [
            { attempt: 1, outcome: "stopped", why: "drone_killed", started_at: "2026-09-10T14:12:27Z", ended_at: KILLED_AT },
            { attempt: 2, outcome: "advanced", started_at: AGAIN_AT, ended_at: "2026-09-10T14:16:07Z" },
          ],
          verdicts: [{ attempt: 2, named: "passed" }],
          deliverables: (step.deliverables ?? []).map((one) => ({
            attempt: 2,
            path: one.path.replace("root_cause.1.", "root_cause.2."),
          })),
        },
  );
  return {
    ...was,
    name: "running — every Drone it has had, one of them killed",
    watched: { ...was.watched, detail: { ...detail, steps } },
    observed: observedWatching(turns, true),
    jobDrones: { state: "read", jobId: JOB_ID, drones: DRONES_HAD },
  };
}
