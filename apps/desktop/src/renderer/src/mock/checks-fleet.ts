// A Fleet whose repository has had Checks requested and run: one out and printing, two waiting
// behind it in Verify, and four ended — passed, failed, stopped. Beside them the run list holds a
// Command and a Setup entry, which the Checks page leaves out. `checks`, the scenario the walk of
// the same name plays.

import type { CheckOutput, CheckoutRunRecord, CheckoutRunSheetRead, CheckoutVerify, ManifestCheckRow, MergeLines } from "@armada/protocol";
import { everyDroneHad } from "@armada/jobs/fixtures/build/drones-had";
import { ARC_DRONES } from "@armada/jobs/fixtures/build/arc-plan";
import { ARC_HANDLE, ARC_JOB_ID, ARC_TITLE } from "@armada/jobs/fixtures/build/arc-base";
import { executingSequential } from "@armada/jobs/fixtures/build/arc";
import { JOB_HANDLE, JOB_ID, repository } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
import { holding } from "./holding";
import { sheet, manifesting } from "./manifest-fake";
import type { FleetHandle, Scenario } from "./moment";

/** How far apart the running Check's lines arrive: inside a walk step's five-second wait. */
const ARRIVING_MS = 2_000;

const TEST = "cargo nextest run --workspace --exclude acceptance";

function ended(
  name: string,
  command: string,
  at: string,
  ms: number,
  over: Partial<CheckoutRunRecord> = {},
): CheckoutRunRecord {
  return {
    id: `crun_${name}`,
    name,
    command,
    required: [],
    started_at: at,
    ended_at: at,
    duration_ms: ms,
    exit_code: 0,
    expect_exit_code: 0,
    ended: "exited 0",
    stopped: false,
    changed: [],
    undoable: false,
    log: `runs/crun_${name}/output.log`,
    ...over,
  };
}

const BUILD = ended("build", "cargo build --workspace --locked", "2026-10-06T14:19:18Z", 41_300, { required: ["bootstrap"] });
const TYPECHECK = ended("typecheck", "pnpm typecheck", "2026-10-06T14:02:30Z", 9_420, { exit_code: 2, ended: "exited 2" });
const STORYBOOK = ended("storybook", "pnpm -C packages/components build-storybook", "2026-10-06T13:51:02Z", 3_000, {
  exit_code: undefined,
  stopped: true,
  ended: "stopped",
});
const BRIDGE_TEST = ended("bridge_test", "pnpm bridge-test", "2026-10-06T13:40:11Z", 128_000, {
  changed: [{ path: "apps/desktop/coverage/index.html", change: "modified" }],
});
/** In the run list and not Checks: a Command and a Setup entry. */
const FMT = ended("fmt", "cargo fmt --all", "2026-10-06T14:20:01Z", 1_900);
const BOOTSTRAP = ended("bootstrap", "pnpm install --frozen-lockfile", "2026-10-06T13:30:11Z", 8_200);

const VERIFY: CheckoutVerify = {
  id: "01VERIFY",
  started_at: "2026-10-06T14:19:02Z",
  steps: [
    { group: "setup", name: "bootstrap", run: BOOTSTRAP.command, state: "ran", record: BOOTSTRAP },
    { group: "checks", name: "build", run: BUILD.command, state: "ran", record: BUILD },
    { group: "checks", name: "test", run: TEST, state: "running", run_id: "crun_test" },
    { group: "checks", name: "format", run: "cargo fmt --all --check", state: "waiting" },
    { group: "checks", name: "bridge_build", run: "pnpm -C apps/desktop build", state: "waiting" },
  ],
};

const READ: CheckoutRunSheetRead = {
  state: "read",
  sheet: sheet({
    running: { id: "crun_test", name: "test", command: TEST, started_at: "2026-10-06T14:20:02Z" },
    verify: VERIFY,
  }),
};

const LOGS: Record<string, string[]> = {
  crun_build: ["   Compiling ipc v0.0.0", "   Compiling fleet v0.0.0", "    Finished `dev` profile in 41.2s"],
  crun_typecheck: ["$ tsc -b", "packages/screens/src/Row.tsx(88,7): error TS2322: Type 'string' is not assignable to type 'number'."],
  crun_storybook: ["$ storybook build", "info => Cleaning outputDir"],
  crun_bridge_test: [" RUN  v4.1.11 /Users/user/armada/apps/desktop", " ✓ src/main/connection.test.ts (18 tests) 211ms", " Test Files  65 passed (65)"],
};

const OUT_NOW = [" Nextest run ID 4a1c in 12 binaries", "        PASS [   0.412s] core-model tests::state_machine"];
const ARRIVING = ["        PASS [   1.904s] fleet tests::reads_the_runtime_file", "        PASS [   0.090s] ipc tests::version_skew"];

/** The line's turn for one branch: its Checks, each as the runner asked for them. */
const LINE: MergeLines = {
  lines: [
    {
      root: repository().root,
      line: [
        {
          branch: "fleet/pulse-log-rows",
          place: 1,
          state: "gating",
          doing: "running its Checks",
          checks: [
            { name: "build", state: "passed", started_at: "2026-10-06T14:19:50Z", requester: { kind: "merge_line", branch: "fleet/pulse-log-rows" } },
            { name: "desktop_test", state: "running", started_at: "2026-10-06T14:20:20Z", requester: { kind: "merge_line", branch: "fleet/pulse-log-rows" } },
            { name: "screens_test", state: "waiting", requester: { kind: "merge_line", branch: "fleet/pulse-log-rows" } },
          ],
        },
      ],
      off: [],
      landed: [],
      sent_back: [],
    },
  ],
};

const LAND_LOGS: Record<string, string[]> = {
  "fleet/pulse-log-rows build": ["   Compiling fleet v0.0.0", "    Finished `dev` profile in 38.0s"],
  "fleet/pulse-log-rows desktop_test": [" RUN  v4.1.11 /Users/user/armada/apps/desktop", " ✓ src/main/connection.test.ts (18 tests) 211ms"],
};

/** Each of these opens a log by its `kept` name, read under the Job as `get_check_output` does. */
const REPRO_DRONE = "01M1HHJ6XB001BZJZ4BE2RPR0A";

/**
 * What `list_manifest_checks` answers, newest first: a gate's Check, a Drone's run on a task, still
 * going, and a Drone's run of two Checks on a step. **A gate row has no start or duration**, only
 * when its ruling was written.
 */
const REPORTED: ManifestCheckRow[] = [
  {
    source: "asked_run",
    requester: { kind: "drone_task", job_id: ARC_JOB_ID, step: "implement", task_id: "T2", drone_id: ARC_DRONES.T2 ?? "T2", handle: ARC_HANDLE },
    job_id: ARC_JOB_ID,
    job_handle: ARC_HANDLE,
    job_title: ARC_TITLE,
    step: "implement",
    attempt: 1,
    name: "scripts_test",
    state: "running",
    started_at: "2026-10-06T14:18:30Z",
    logs: [{ check: "scripts_test", kept: "fix.1.ask.scripts_test.log" }],
    asked_run_id: 17,
  },
  {
    source: "gate",
    requester: { kind: "gate", job_id: JOB_ID, step: "root_cause", handle: JOB_HANDLE },
    job_id: JOB_ID,
    job_handle: JOB_HANDLE,
    job_title: "Split the settings reducer",
    step: "root_cause",
    attempt: 2,
    name: "components_test",
    state: "passed",
    ended_at: "2026-10-06T13:23:10Z",
    logs: [{ check: "components_test", kept: "root_cause.2.components_test.log" }],
  },
  {
    source: "asked_run",
    requester: { kind: "drone_step", job_id: JOB_ID, step: "repro", drone_id: REPRO_DRONE, handle: JOB_HANDLE },
    job_id: JOB_ID,
    job_handle: JOB_HANDLE,
    job_title: "Split the settings reducer",
    step: "repro",
    attempt: 1,
    name: "hooks_test, format",
    state: "failed",
    started_at: "2026-10-06T13:05:00Z",
    ended_at: "2026-10-06T13:05:09Z",
    took_ms: 9_000,
    logs: [
      { check: "hooks_test", kept: "repro.1.ask.hooks_test.log" },
      { check: "format", kept: "repro.1.ask.format.log" },
    ],
    asked_run_id: 9,
  },
];

/** The Job's Check logs, by `kept`. */
const JOB_LOGS: Record<string, string[]> = {
  "root_cause.2.components_test.log": ["$ vitest run", " ✓ src/Sheet.stories.tsx (14 tests) 211ms", " Test Files  182 passed (182)"],
  "repro.1.ask.hooks_test.log": ["$ python3 -m unittest", "Ran 6 tests in 0.04s", "OK"],
  "repro.1.ask.format.log": ["$ cargo fmt --all --check", "Diff in crates/fleet/src/lib.rs:12:"],
  "fix.1.ask.scripts_test.log": ["$ python3 -m unittest scripts/test_land.py", "test_preflight (test_land.Land) ... ok"],
};

export function checking(): Scenario {
  return checkingOver(READ, "checks", "A repository with Checks out, waiting and ended, who asked for each, and the logs behind them");
}

/** The same Fleet with the run sheet never read: what the gate's and the Drones' Checks and the merge line's still draw. */
export function checkingWithoutRunSheet(): Scenario {
  return checkingOver({ state: "reading" }, "checks-without-run-sheet", "A repository whose run sheet has not been read, and the Checks Jobs and Drones reported anyway");
}

function checkingOver(read: CheckoutRunSheetRead, name: string, says: string, reported: ManifestCheckRow[] = REPORTED, jobLogs: Record<string, string[]> = JOB_LOGS): Scenario {
  const base = manifesting({ sheet: read, runs: { runs: [FMT, BUILD, TYPECHECK, STORYBOOK, BRIDGE_TEST, BOOTSTRAP], unreadable: [] } });
  // The Job the gate's and the Drones' requesters open, with its steps and Drones, and one with a plan, which a task's requester opens.
  const jobs = holding("checks", "", [everyDroneHad(), ...executingSequential().fixtures]);
  return {
    ...base,
    name,
    state: { ...jobs.state, repository: base.state.repository, mergeLines: LINE },
    reads: jobs.reads,
    says,
    behaves: (fleet) => ({ ...base.behaves?.(fleet), ...printing(fleet), ...jobChecks(fleet, reported, jobLogs) }),
  };
}

/** The followed run's lines as they arrive, and an ended run's whole log. */
function printing(fleet: FleetHandle): Partial<BridgeApi> {
  let timers: ReturnType<typeof setTimeout>[] = [];
  const stop = () => {
    timers.forEach(clearTimeout);
    timers = [];
  };
  const show = (lines: string[]) =>
    fleet.publish({
      checkoutRunFollowed: { state: "following", runId: "crun_test", name: "test", path: ".armada/runs/crun_test/output.log", fromLine: 1, lines },
    });
  return {
    observeCheckoutRun: async (runId) => {
      stop();
      if (runId !== "crun_test") {
        fleet.publish({ checkoutRunFollowed: { state: "none" } });
        return;
      }
      show(OUT_NOW);
      ARRIVING.forEach((_, n) => {
        timers.push(setTimeout(() => show([...OUT_NOW, ...ARRIVING.slice(0, n + 1)]), ARRIVING_MS * (n + 1)));
      });
    },
    followLandCheck: async (at) => {
      const lines = at === null ? undefined : LAND_LOGS[`${at.branch} ${at.check}`];
      if (at === null || lines === undefined) {
        fleet.publish({ landFollowed: { state: "none" } });
        return;
      }
      fleet.publish({
        landFollowed: { state: "following", root: at.root, branch: at.branch, check: at.check, fromLine: 1, lines, ...(at.check === "build" ? { ended: "finished" } : {}) },
      });
    },
    getCheckoutRunOutput: async (runId) => {
      const lines = LOGS[runId];
      return lines === undefined
        ? { ok: false, outcome: { ok: false, why: "not_connected" } }
        : { ok: true, output: { id: runId, name: runId.replace("crun_", ""), path: `.armada/runs/${runId}/output.log`, lines, from_line: 1, total_lines: lines.length, bytes: 0, whole: true } };
    },
  };
}

/** `list_manifest_checks`, and the Job's own Check logs: read whole, and followed on the running one. */
function jobChecks(fleet: FleetHandle, reported: ManifestCheckRow[], logs: Record<string, string[]>): Partial<BridgeApi> {
  return {
    readManifestChecks: async () => ({ ok: true, checks: { rows: reported, total: reported.length } }),
    readCheckOutput: async (_jobId, kept) => {
      const lines = logs[kept];
      if (lines === undefined) return { ok: false, outcome: { ok: false, why: "not_connected" } };
      const output: CheckOutput = { attempt: 1, name: kept, path: `.armada/${kept}`, lines, from_line: 1, total_lines: lines.length, bytes: 0, whole: true };
      return { ok: true, output };
    },
    followCheckOutput: async (jobId, kept) => {
      const lines = kept === null ? undefined : logs[kept];
      if (jobId === null || kept === null || lines === undefined) {
        fleet.publish({ followed: { state: "none" } });
        return;
      }
      fleet.publish({ followed: { state: "following", jobId, kept, name: kept, attempt: 1, path: `.armada/${kept}`, fromLine: 1, lines } });
    },
  };
}

const SPLIT_BRANCH = "fleet/settings-reducer";
const SPLIT_LINE = { kind: "merge_line", branch: SPLIT_BRANCH, job_id: JOB_ID, handle: JOB_HANDLE } as const;

/** One of this Job's rows on `list_manifest_checks`; each says what is its own. */
function onJob(row: Pick<ManifestCheckRow, "source" | "requester" | "step" | "name" | "state"> & Partial<ManifestCheckRow>): ManifestCheckRow {
  return { job_id: JOB_ID, job_handle: JOB_HANDLE, job_title: "Split the settings reducer", attempt: 1, ...row };
}

/**
 * The Checks of the Job `job-checks` opens, asked from its gate and from a Drone on a step. The
 * rows of `REPORTED` that belong to another Job stay, so a narrowing has something to leave out.
 */
const JOB_CHECKS: ManifestCheckRow[] = [
  onJob({
    source: "asked_run",
    requester: { kind: "drone_step", job_id: JOB_ID, step: "repro", drone_id: REPRO_DRONE, handle: JOB_HANDLE },
    step: "repro",
    attempt: 2,
    name: "hooks_test",
    state: "running",
    started_at: "2026-10-06T14:20:10Z",
    logs: [{ check: "hooks_test", kept: "repro.2.ask.hooks_test.log" }],
    asked_run_id: 21,
  }),
  onJob({
    source: "asked_run",
    requester: { kind: "drone_step", job_id: JOB_ID, step: "repro", drone_id: REPRO_DRONE, handle: JOB_HANDLE },
    step: "repro",
    name: "format",
    state: "failed",
    started_at: "2026-10-06T14:05:00Z",
    ended_at: "2026-10-06T14:05:09Z",
    took_ms: 9_000,
    logs: [{ check: "format", kept: "repro.1.ask.format.log" }],
    asked_run_id: 20,
  }),
  // A gate Check waiting on another in its gate: no start and no end, as Fleet reports it.
  onJob({
    source: "gate",
    requester: { kind: "gate", job_id: JOB_ID, step: "root_cause", handle: JOB_HANDLE },
    step: "root_cause",
    attempt: 2,
    name: "desktop_test",
    state: "waiting",
  }),
  onJob({
    source: "gate",
    requester: { kind: "gate", job_id: JOB_ID, step: "root_cause", handle: JOB_HANDLE },
    step: "root_cause",
    attempt: 2,
    name: "components_test",
    state: "passed",
    ended_at: "2026-10-06T13:23:10Z",
    logs: [{ check: "components_test", kept: "root_cause.2.components_test.log" }],
  }),
  onJob({
    source: "gate",
    requester: { kind: "gate", job_id: JOB_ID, step: "root_cause", handle: JOB_HANDLE },
    step: "root_cause",
    attempt: 2,
    name: "docs_test",
    state: "skipped",
    ended_at: "2026-10-06T13:23:09Z",
  }),
  onJob({
    source: "gate",
    requester: { kind: "gate", job_id: JOB_ID, step: "root_cause", handle: JOB_HANDLE },
    step: "root_cause",
    name: "clippy",
    state: "failed",
    ended_at: "2026-10-06T12:58:40Z",
    logs: [{ check: "clippy", kept: "root_cause.1.clippy.log" }],
  }),
  ...REPORTED.filter((row) => row.job_id === ARC_JOB_ID),
];

const JOB_CHECK_LOGS: Record<string, string[]> = {
  ...JOB_LOGS,
  "repro.2.ask.hooks_test.log": ["$ python3 -m unittest hooks", "test_dispatch (test_hooks.Hooks) ... ok"],
  "repro.1.ask.format.log": ["$ cargo fmt --all --check", "Diff in crates/fleet/src/lib.rs:12:"],
  "root_cause.1.clippy.log": ["$ tsc -b", "packages/screens/src/Row.tsx(88,7): error TS2322: Type 'string' is not assignable to type 'number'."],
};

/** The merge line's Checks for the Job's branch, one passed and one waiting. **Each names the Job**, as Fleet sends it since 23.64. */
const JOB_LINE: MergeLines = {
  lines: [
    {
      root: repository().root,
      line: [
        {
          branch: SPLIT_BRANCH,
          place: 1,
          state: "gating",
          doing: "running its Checks",
          checks: [
            { name: "ipc_test", state: "passed", started_at: "2026-10-06T14:19:50Z", requester: SPLIT_LINE },
            { name: "fleet_test", state: "waiting", requester: SPLIT_LINE },
          ],
        },
      ],
      off: [],
      landed: [],
      sent_back: [],
    },
  ],
};

/**
 * `job-checks`: a Job open, with Checks in every state, asked from its gate, a Drone and the merge
 * line. The Checks page holds them with another Job's and the checkout's own.
 */
export function jobChecking(): Scenario {
  const one = checkingOver(READ, "job-checks", "A Job whose Checks are running, waiting, passed, failed and skipped, from three requesters", JOB_CHECKS, JOB_CHECK_LOGS);
  return { ...one, opens: JOB_ID, state: { ...one.state, mergeLines: JOB_LINE } };
}
