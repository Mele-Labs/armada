// A Session whose agent ran three Checks in its slot: one still out, one passed, one failed. Its
// thread draws a row for each, and the Checks page lists them beside the Jobs' runs with the
// Session's owner chip. The walk `session-checks` plays it; `later` fails the one that was out.

import type { ManifestCheckRow } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { checkingOver, REPORTED } from "../checks-fleet";
import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { s200Sessions } from "./sessions";

const SLOT = 10;
const ID = "s9";
const OUT = 31;

/** Whether the Check that was out has ended, shared by the thread and the Checks page. */
const ended = { failed: false };

const run = (n: number, name: string, state: "running" | "passed" | "failed", at: string): ManifestCheckRow => ({
  source: "asked_run",
  requester: { kind: "session", session_id: ID, slot: SLOT },
  session_id: ID,
  attempt: 1,
  name,
  state,
  started_at: `2026-10-06T${at}Z`,
  ...(state === "running" ? {} : { ended_at: `2026-10-06T${at}Z`, took_ms: 21_000 }),
  logs: [{ check: name, kept: `session.${ID}.${n}.log` }],
  asked_run_id: n,
});

const rows = (): ManifestCheckRow[] => [
  run(OUT, "app_smoke", ended.failed ? "failed" : "running", "14:23:10"),
  run(30, "components_test", "failed", "14:22:20"),
  run(29, "desktop:typecheck", "passed", "14:21:40"),
  ...REPORTED,
];

const LOGS: Record<string, string[]> = {
  [`session.${ID}.${OUT}.log`]: ["$ vitest run --project app_smoke", " FAIL  src/renderer/src/mock/walks.test.tsx", "AssertionError: expected the sheet to be visible"],
  [`session.${ID}.30.log`]: ["$ vitest run --project components_test", " FAIL  SessionThread.stories.tsx > Check rows", "AssertionError: expected 1 to be 2"],
  [`session.${ID}.29.log`]: ["$ tsc -b", "Found 0 errors."],
};

const checked = (): Session => ({
  id: ID,
  title: "Session check rows",
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  lastTurn: "14:23",
  rows: [
    { id: "c1", at: "14:20:50", kind: "message", from: { kind: "you" }, text: "Run the checks on the thread rows." },
    { id: "c2", at: "14:21:20", kind: "tool", text: "Edit packages/components/src/compositions/SessionThread/SessionThread.tsx" },
    { id: "c3", at: "14:21:40", kind: "check", name: "desktop:typecheck", run: 29, state: "passed" },
    { id: "c4", at: "14:22:20", kind: "check", name: "components_test", run: 30, state: "failed" },
    { id: "c5", at: "14:23:10", kind: "check", name: "app_smoke", run: OUT, state: ended.failed ? "failed" : "running" },
  ],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: "sessions/session-checks", slot: SLOT },
  ],
});

/** The store with the Session added, whose last Check ends when time passes. */
function store(board: Parameters<NonNullable<NonNullable<Scenario["draft"]>["sessions"]>>[0]): SessionsStore {
  const inner = s200Sessions.draft!.sessions!(board) as SessionsStore;
  const listeners = new Set<() => void>();
  let seen: readonly Session[] | undefined;
  let drawn: readonly Session[] = [];
  let mine = checked();
  return {
    ...inner,
    get: () => {
      const now = inner.get();
      if (now !== seen) {
        seen = now;
        drawn = [...now, mine];
      }
      return drawn;
    },
    subscribe: (listener) => {
      listeners.add(listener);
      const off = inner.subscribe(listener);
      return () => {
        listeners.delete(listener);
        off();
      };
    },
    later: () => {
      inner.later();
      ended.failed = true;
      mine = checked();
      seen = undefined;
      listeners.forEach((one) => one());
    },
  };
}

const over = checkingOver(
  { state: "reading" },
  "session-checks",
  "A Session's agent ran three Checks in its slot: one out, one passed, one failed, listed on the Checks page with the Session's owner chip",
);

export const s203SessionChecks: Scenario = {
  ...over,
  held: s200Sessions.held,
  behaves: (fleet) => {
    ended.failed = false;
    const base = over.behaves?.(fleet) ?? {};
    return {
      ...base,
      readManifestChecks: async () => ({ ok: true, checks: { rows: rows(), total: rows().length } }),
      readSessionCheckOutput: async (run) => {
        const kept = `session.${ID}.${run}.log`;
        const lines = LOGS[kept];
        return lines === undefined
          ? { ok: false, outcome: { ok: false, why: "not_connected" } }
          : { ok: true, output: { attempt: 1, name: kept, path: `session-check/${run}`, lines, from_line: 1, total_lines: lines.length, bytes: 0, whole: true } };
      },
    };
  },
  draft: { sessions: store },
};
