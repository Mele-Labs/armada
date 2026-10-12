// The Sessions list with three hundred ended Sessions. The Ended view draws the first fifty and the next
// fifty each time its end scrolls into view.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const WORK = ["Trim the retry loop", "Fix the flaky store test", "Draft the migration notes", "Prune the stale branches", "Why the reader drops lines", "Rename the slot pool"];

const records = () => {
  const start = Date.now();
  return [
    hosted("01IDLECCCCCCCCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: new Date(start).toISOString() }),
    ...Array.from({ length: 300 }, (_, n) => {
      const at = new Date(start - (n + 1) * 15 * 60_000).toISOString();
      return hosted(`01ENDED${String(n).padStart(19, "0")}`, { title: `${WORK[n % WORK.length]} ${n + 1}`, state: "ended", last_seen_at: at, last_turn_at: at });
    }),
  ];
};

function build(): Scenario {
  const board = onBoard([], { repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "session-list-ended-many",
    says: "Three hundred ended Sessions, drawn fifty at a time as the Ended view scrolls",
    behaves: (handle) => new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle),
  };
}

export const sessionListEndedMany: Scenario = build();
