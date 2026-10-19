// Four Sessions, each holding a pull request whose checks stand differently: the required check red,
// a shard red while the run goes on, a shard red after the required checks passed, and all green.
// Over the `sessions` scenario. The walk `session-checks-weights` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const held = (id: string, title: string, number: number, checks: Extract<Session["attachments"][number], { kind: "pull_request" }>["checks"], at: number): Session => ({
  id,
  title,
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(at),
  rows: [{ id: `${id}-1`, at: "22:18:54", kind: "message", from: { kind: "agent" }, text: `The change is up as #${number}.` }],
  attachments: [
    { kind: "slot", slot: at },
    {
      kind: "pull_request",
      number,
      title,
      branch: `armada/${id}`,
      address: `https://example.com/pull/${number}`,
      checks,
      state: "open",
      auto: false,
    },
  ],
});

const sessions: Session[] = [
  held("s31", "Gate failed", 2301, { state: "failed", failing: "ci" }, 2),
  held("s32", "Shard red, run going", 2302, { state: "pending", red: "app_smoke_shard (2)" }, 3),
  held("s33", "Shard red, gate passed", 2303, { state: "passed", red: "app_smoke_shard (2)" }, 4),
  held("s34", "All green", 2304, { state: "passed" }, 5),
];

export const s207SessionChecksWeights: Scenario = {
  ...s200Sessions,
  name: "session-checks-weights",
  says: "Four Sessions: the required check red, a shard red mid-run, a shard red after the gate passed, and green",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, sessions) },
};
