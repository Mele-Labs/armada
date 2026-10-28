// A Job whose title is the long first line of a pinned note, with the wand after it that has Fleet name it. The fake
// Fleet answers after a second with a short name. The walk `name-it-for-me` plays it.

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const AT = "2026-10-15T01:20:00.000Z";
/** The id `NAMED_FOR_ME` has a name for. */
const ID = "7c41d9a2-3e58-4b1f-a0d6-91f2c5e8b370";
const NOTE = "I think the Checks tab should sit beside Bays and open in the spine instead of";

const records = () => [
  hosted(ID, {
    title: NOTE,
    started_at: AT,
    last_turn_at: AT,
    attachments: [held("slot", "3"), held("branch", "move-checks-tab")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: true },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [ID]: [
    { kind: "message", id: "m1", at: AT, from: { kind: "you" }, text: `${NOTE} a sheet over the thread.` },
    { kind: "message", id: "m2", at: AT, from: { kind: "agent" }, text: "Reading where the tabs are drawn." },
  ],
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return { ...served, name: "name-it-for-me", says: "A Job titled with the long first line of a note, and the wand that has Fleet name it" };
}

export const nameItForMe: Scenario = build();
