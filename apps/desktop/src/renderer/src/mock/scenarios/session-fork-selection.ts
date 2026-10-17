// A live Session forked from words selected in an earlier message. The Session keeps running, and the
// new one opens on the conversation up to that message with the words in its box as a quote. The walk
// `session-fork-selection` plays it, over a Fleet that forks as `session-fork` does (`../sessions-fleet.ts`).

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-07T13:48:02.000Z";
const ID = "01SESSIONAAAAAAAAAAAAAAAAA";

const records = () => [
  hosted(ID, { title: "Fix the flaky store test", attachments: [held("slot", "3"), held("branch", "fix/flaky-store")] }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [ID]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: "store_open fails about one run in ten. Find out why." },
    { kind: "tool", id: "t1", at: AT, text: "Read crates/store/src/clock.rs" },
    { kind: "tool", id: "t2", at: AT, text: "Bash cargo test -p store store_open" },
    {
      kind: "message",
      id: "m2",
      at: AT,
      from: { kind: "agent" },
      text: "The test builds its expected timestamp from the wall clock, and the store stamps the row from its own read of it. When the second rolls over between the two reads, the values differ by one second. The two reads sit about 100 ms apart, which is the one run in ten.",
    },
    { kind: "message", id: "m3", at: AT, from: { kind: "you" }, text: "Is that the only place it reads the clock?" },
    { kind: "tool", id: "t3", at: AT, text: "Bash rg SystemTime::now crates/store" },
    {
      kind: "message",
      id: "m4",
      at: AT,
      from: { kind: "agent" },
      text: "No. Two more: the retention sweep in sweep.rs and the helper in fixtures.rs. Only the store's read is under test. I would pin the clock there and leave the other two.",
    },
  ],
});

function build(): Scenario {
  const board = onBoard({
    repositories: [ARMADA],
    picked: ARMADA.root,
  });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "session-fork-selection",
    says: "A live Session, forked from words selected in an earlier message",
    // A Fleet of its own for each window, so a walk told twice starts from the same Session.
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s203SessionForkSelection: Scenario = build();
