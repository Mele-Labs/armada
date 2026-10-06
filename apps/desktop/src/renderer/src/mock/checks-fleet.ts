// A Fleet whose repository has had Checks requested and run: one out and printing, two waiting
// behind it in Verify, and four ended — passed, failed, stopped. Beside them the run list holds a
// Command and a Setup entry, which the Checks page leaves out. `checks`, the scenario the walk of
// the same name plays.

import type { CheckoutRunRecord, CheckoutRunSheetRead, CheckoutVerify, MergeLines } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import type { BridgeApi } from "../../../shared/api";
import { sheet, manifesting } from "./manifest-fleet";
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
            { name: "build", state: "passed", requester: { kind: "merge_line", branch: "fleet/pulse-log-rows" } },
            { name: "desktop_test", state: "running", requester: { kind: "merge_line", branch: "fleet/pulse-log-rows" } },
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

export function checking(): Scenario {
  const base = manifesting({ sheet: READ, runs: { runs: [FMT, BUILD, TYPECHECK, STORYBOOK, BRIDGE_TEST, BOOTSTRAP], unreadable: [] } });
  return {
    ...base,
    name: "checks",
    state: { ...base.state, mergeLines: LINE },
    says: "A repository with Checks out, waiting and ended, and the logs behind them",
    behaves: (fleet) => ({ ...base.behaves?.(fleet), ...printing(fleet) }),
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
