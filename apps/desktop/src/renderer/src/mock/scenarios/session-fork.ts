// A Session that is dead, forked. A live Session from Bridge, and a Session from a terminal whose mod
// has stopped asking: the first offers a message box and no Fork, the second a Fork and no box. The
// walk `session-fork` plays it, over a Fleet that forks as Fleet does (`../sessions-fleet.ts`).

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, terminal } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-07T13:48:02.000Z";

const records = () => [
  hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }),
  terminal("01TERMINALBBBBBBBBBBBBBBBB", { title: "Why the reader drops lines", terminal: {} }),
  hosted("01ENDEDCCCCCCCCCCCCCCCCCCC", { title: "Trim the retry loop", state: "ended", last_seen_at: new Date().toISOString() }),
];

const threads = (): Record<string, SessionRow[]> => ({
  "01TERMINALBBBBBBBBBBBBBBBB": [
    { kind: "message", id: "t1", at: AT, from: { kind: "you" }, text: "Why does the reader drop the last line of a stream?" },
    { kind: "tool", id: "t2", at: AT, text: "Read crates/adapters/src/reading.rs" },
    { kind: "message", id: "t3", at: AT, from: { kind: "agent" }, text: "It splits on newlines and keeps nothing after the last one." },
  ],
});

function build(): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "session-fork",
    says: "A Session whose terminal went quiet, forked into a new one",
    // A Fleet of its own for each window, so a walk told twice starts from the same two Sessions.
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s203SessionFork: Scenario = build();
