// The Sessions list in its views. One Session in every state: asking, working, idle, not started, quiet and
// ended. The walk `session-list-views` plays it: Active on opening, then Quiet, Ended and All.

import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { asking, FakeSessionsFleet, held, hosted, terminal } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const now = () => new Date().toISOString();

const records = () => [
  hosted("01ASKINGAAAAAAAAAAAAAAAAAA", { title: "Prune the stale branches", last_turn_at: now(), hosted: { turn: { state: "idle" }, mode: "auto", running: true, asked: asking("git push --force origin prune/stale") } }),
  hosted("01WORKINGBBBBBBBBBBBBBBBBB", { title: "Draft the migration notes", last_turn_at: now(), hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
  hosted("01IDLECCCCCCCCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: now(), attachments: [held("slot", "3")] }),
  hosted("01BLANKDDDDDDDDDDDDDDDDDDD"),
  terminal("01QUIETEEEEEEEEEEEEEEEEEEE", { title: "Why the reader drops lines", last_turn_at: now(), terminal: {} }),
  hosted("01ENDEDFFFFFFFFFFFFFFFFFFF", { title: "Trim the retry loop", state: "ended", last_seen_at: now() }),
];

function build(): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "session-list-views",
    says: "The Sessions list, Active on opening, with Quiet, Ended and All beside it",
    behaves: (handle) => new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle),
  };
}

export const s204SessionListViews: Scenario = build();
