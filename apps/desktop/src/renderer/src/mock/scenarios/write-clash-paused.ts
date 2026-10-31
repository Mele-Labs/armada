// A Job whose changes conflict with a paused Job's: Fleet's write-clash notice names the paused Job's branch and says to
// resolve the conflict on merge, since a message to a paused Job waits for Resume. The walk `write-clash-paused` plays it.

import type { RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const JOB = "355c64b4-88fc-4181-93c6-99d1f811f88f";
const PAUSED = "4eb87a78-1ef3-4fe7-a0b1-0445d7715c72";
const FLEET = { kind: "session", id: "fleet", title: "Fleet" } as const;
const NOTICE =
  "Your changes to `apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx` conflict with branch mock/video-multitasking-boards. Its Job s-4eb87a78 is paused, so resolve the conflict on merge.";

const records = () => [
  hosted(JOB, { title: "Thread, Browser and Terminal Split", attachments: [held("slot", "1"), held("branch", "armada/session-355c64b4")] }),
  hosted(PAUSED, {
    title: "Demo marketing video",
    attachments: [held("branch", "mock/video-multitasking-boards")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: false, paused: { at: "2026-10-30T17:38:55.820Z", resuming: false } },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [JOB]: [
    { kind: "message", id: "m1", at: "2026-10-31T01:02:00.000Z", from: { kind: "you" }, text: "Split the Job view into Thread, Browser and Terminal." },
    { kind: "tool", id: "t1", at: "2026-10-31T01:05:00.000Z", text: "Edit apps/desktop/src/renderer/src/cockpit/deck/Deck.tsx" },
    { kind: "message", id: "n1", at: "2026-10-31T01:10:07.970Z", from: FLEET, text: NOTICE, notice: "write_clash" },
  ],
  [PAUSED]: [],
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "write-clash-paused",
    says: "A Job's changes conflict with a paused Job's: Fleet's notice names the branch and says to resolve it on merge",
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s300WriteClashPaused: Scenario = build();
