// A Session whose agent has two Checks out and one that passed before them, the turn ended and the
// thread moved on. The header holds a mark with the running count, and a press lists them. Over
// `session-checks`. The walk `session-running-checks` plays it.

import type { ManifestCheckRow } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { s203SessionChecks } from "./session-checks";

const SLOT = 10;
const ID = "s9";
const PASSED = 29;
const TEST = 32;
const SMOKE = 33;

/** The wall clock `seconds` ago, as the thread's rows say it: "14:23:10". */
const clock = (seconds: number): string => new Date(Date.now() - seconds * 1000).toTimeString().slice(0, 8);

const run = (n: number, name: string, state: "running" | "passed", seconds: number, target?: string): ManifestCheckRow => ({
  source: "asked_run",
  requester: { kind: "session", session_id: ID, slot: SLOT },
  session_id: ID,
  attempt: 1,
  name,
  state,
  started_at: new Date(Date.now() - seconds * 1000).toISOString(),
  ...(state === "running" ? {} : { ended_at: new Date().toISOString(), took_ms: 21_000 }),
  ...(target === undefined ? {} : { target }),
  logs: [{ check: name, kept: `session.${ID}.${n}.log` }],
  asked_run_id: n,
});

const LOGS: Record<string, string[]> = {
  [`session.${ID}.${TEST}.log`]: ["$ cargo nextest run -p session_checking", "   Compiling session_checking v0.1.0", "        PASS [   0.012s] session_checking::marks::counts_running"],
  [`session.${ID}.${SMOKE}.log`]: ["$ vitest run --project app_smoke", " RUN  v3.2.4"],
  [`session.${ID}.${PASSED}.log`]: ["$ tsc -b", "Found 0 errors."],
};

const running = (): Session => ({
  id: ID,
  title: "Session running checks",
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  lastTurn: clock(0).slice(0, 5),
  rows: [
    { id: "r1", at: clock(420), kind: "message", from: { kind: "you" }, text: "Check the running count in the header, then smoke the app." },
    { id: "r2", at: clock(360), kind: "tool", text: "Edit packages/components/src/compositions/SessionFrame/SessionFrame.tsx" },
    { id: "r3", at: clock(300), kind: "check", name: "desktop:typecheck", run: PASSED, state: "passed" },
    { id: "r4", at: clock(190), kind: "check", name: "test", run: TEST, state: "running", target: "session_checking" },
    { id: "r5", at: clock(40), kind: "check", name: "app_smoke", run: SMOKE, state: "running" },
  ],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: "sessions/session-running-checks", slot: SLOT },
  ],
});

function store(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]): SessionsStore {
  const inner = s203SessionChecks.draft!.sessions!(board) as SessionsStore;
  const mine = running();
  let seen: readonly Session[] | undefined;
  let drawn: readonly Session[] = [];
  return {
    ...inner,
    get: () => {
      const now = inner.get();
      if (now !== seen) {
        seen = now;
        drawn = [...now.filter((one) => one.id !== ID), mine];
      }
      return drawn;
    },
  };
}

export const s266SessionRunningChecks: Scenario = {
  ...s203SessionChecks,
  name: "session-running-checks",
  says: "A Session's agent has two Checks out and the thread has moved on: the header counts them, and a press lists them",
  behaves: (fleet) => {
    const base = s203SessionChecks.behaves?.(fleet) ?? {};
    const rows = [run(SMOKE, "app_smoke", "running", 40), run(TEST, "test", "running", 190, "session_checking"), run(PASSED, "desktop:typecheck", "passed", 300)];
    return {
      ...base,
      readManifestChecks: async () => ({ ok: true, checks: { rows, total: rows.length } }),
      readSessionCheckOutput: async (n) => {
        const kept = `session.${ID}.${n}.log`;
        const lines = LOGS[kept];
        return lines === undefined
          ? { ok: false, outcome: { ok: false, why: "not_connected" } }
          : { ok: true, output: { attempt: 1, name: kept, path: `session-check/${n}`, lines, from_line: 1, total_lines: lines.length, bytes: 0, whole: true } };
      },
      followCheckOutput: async (jobId, kept) => {
        const lines = kept === null ? undefined : LOGS[kept];
        if (jobId === null || kept === null || lines === undefined) {
          fleet.publish({ followed: { state: "none" } });
          return;
        }
        fleet.publish({ followed: { state: "following", jobId, kept, name: kept, attempt: 1, path: `session-check/${kept}`, fromLine: 1, lines } });
      },
    };
  },
  draft: { sessions: store },
};
