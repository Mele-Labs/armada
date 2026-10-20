// A Job whose agent disputed a notice of Fleet's: Fleet's card carries the Disputed mark, and the line under the agent's reply
// says why, how it was spotted, and opens the review Job Fleet started in the armada repository. The walk
// `fleet-notice-disputed` plays it, over a Fleet that serves the rows as the wire carries them (`../sessions-fleet.ts`).

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const JOB = "01DISPUTEDJOBAAAAAAAAAAAAA";
const REVIEW = "01DISPUTEREVIEWBBBBBBBBBBB";
const NOTICE = "Subagents burned 5.0 units in your last hour, 87% of it, on opus. Pass model sonnet on reading and searching agents.";
const REPLY = "My reading agents already run on sonnet. The opus spend came from building agents, which write the code.";
const FLEET = { kind: "session", id: "fleet", title: "Fleet" } as const;

const records = () => [
  hosted(JOB, { title: "Port the store to the new clock", attachments: [held("slot", "3"), held("branch", "fix/store-clock")] }),
  hosted(REVIEW, { title: "Review a disputed Fleet notice", started_at: "2026-10-19T14:11:00.000Z" }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [JOB]: [
    { kind: "message", id: "m1", at: "2026-10-19T13:20:00.000Z", from: { kind: "you" }, text: "Port the store to the new clock and keep the tests under a second." },
    { kind: "tool", id: "t1", at: "2026-10-19T13:20:20.000Z", text: "Agent Read the store's clock calls" },
    { kind: "tool", id: "t2", at: "2026-10-19T13:41:00.000Z", text: "Agent Write the clock port" },
    { kind: "message", id: "n1", at: "2026-10-19T14:10:00.000Z", from: FLEET, text: NOTICE },
    { kind: "message", id: "m2", at: "2026-10-19T14:10:40.000Z", from: { kind: "agent" }, text: REPLY },
    { kind: "disputed", id: "d1", at: "2026-10-19T14:10:41.000Z", notice: "n1", why: "Reading agents already run on sonnet. The opus spend came from building agents.", spotted: "called", review: REVIEW },
  ],
  [REVIEW]: [
    {
      kind: "message",
      id: "r1",
      at: "2026-10-19T14:11:00.000Z",
      from: FLEET,
      text: `Port the store to the new clock disputed this notice.\n\nNotice: ${NOTICE}\n\nReason: Reading agents already run on sonnet. The opus spend came from building agents.`,
    },
  ],
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "fleet-notice-disputed",
    says: "A Job's agent disputed a notice of Fleet's: the mark on Fleet's card, the reason, and the review Job it opens",
    // A Fleet of its own for each window, so the walk starts from the same two Jobs.
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s299FleetNoticeDisputed: Scenario = build();
