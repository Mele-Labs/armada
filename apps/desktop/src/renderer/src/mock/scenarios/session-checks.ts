// A Session whose agent ran three Checks in its slot: one still out, one passed, one failed. Its
// thread draws a row for each. Its own turn is idle while a subagent it dispatched runs, as Fleet reads it.
// The walks `session-checks` and `session-checks-while-a-subagent-works` play it; `later` fails the one that was out.

import type { Session } from "@armada/screens/src/draft/sessions";

import { checkingOver } from "../checks-fleet";
import type { Scenario } from "../moment";
import type { SessionsStore } from "../sessions/script";
import { s200Sessions } from "./sessions";

const SLOT = 10;
const ID = "s9";
const OUT = 31;

/** Whether the Check that was out has ended. */
const ended = { failed: false };

const checked = (): Session => ({
  id: ID,
  title: "Session check rows",
  model: "sonnet",
  effort: "medium",
  mode: "auto",
  turn: { state: "idle" },
  standing: "working",
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
    { kind: "subagent", id: "sub-1", task: "Build the Fleet side of Kit's install", state: "running", agentType: "rust-engineer" },
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
  "A Session's agent ran three Checks in its slot: one out, one passed, one failed, each a row in its thread",
);

export const s203SessionChecks: Scenario = {
  ...over,
  held: s200Sessions.held,
  behaves: (fleet) => {
    ended.failed = false;
    return over.behaves?.(fleet) ?? {};
  },
  draft: { sessions: store },
};
