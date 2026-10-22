// A Job's thread where turns Fleet started fold into one dim row each: a merge main notice and a slot notice, handled. Each
// turn's last agent message, and the owner's reply after a notice, draw outside the fold. A third turn ends in a question for
// the owner and stays open. The walk `fleet-turn-fold` plays it.

import type { HelmCallInFlight, RepositorySummary, SessionRow } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const JOB = "01FOLDEDTURNJOBAAAAAAAAAA";
const FLEET = { kind: "session", id: "fleet", title: "Fleet" } as const;

const ASKED: HelmCallInFlight = {
  call: "fold-q1",
  manifest_id: "armada",
  asked_at: "2026-10-22T11:20:00.000Z",
  tool: "AskUserQuestion",
  detail: "",
  truncated: false,
  rule: "AskUserQuestion",
  offers: ["allow_once", "refuse"],
  holding_for_seconds: 600,
  questions: [
    {
      question: "Rebase the branch or merge main into it?",
      header: "Main moved",
      multi_select: false,
      options: [
        { label: "Merge main", description: "Keeps the branch history as it is." },
        { label: "Rebase", description: "Rewrites the branch onto main." },
      ],
    },
  ],
};

const records = () => [
  hosted(JOB, {
    title: "Move the store to the new clock",
    attachments: [held("slot", "2"), held("branch", "armada/session-82490574")],
    hosted: { turn: { state: "idle" }, mode: "auto", running: false, asked: ASKED },
  }),
];

const threads = (): Record<string, SessionRow[]> => ({
  [JOB]: [
    { kind: "message", id: "m1", at: "2026-10-22T10:00:00.000Z", from: { kind: "you" }, text: "Move the store to the new clock and keep the tests under a second." },
    { kind: "tool", id: "t1", at: "2026-10-22T10:01:00.000Z", text: "Read the store's clock calls" },
    { kind: "message", id: "m2", at: "2026-10-22T10:20:00.000Z", from: { kind: "agent" }, text: "The clock port is written. Running the store tests next." },
    { kind: "message", id: "n1", at: "2026-10-22T10:30:00.000Z", from: FLEET, text: "slot-2 (branch armada/session-82490574) is 24 commits behind main. Merge main into your branch before your next push.", notice: "behind_base" },
    { kind: "tool", id: "t0", at: "2026-10-22T10:30:10.000Z", text: "Bash git diff --stat HEAD...main" },
    { kind: "message", id: "m2a", at: "2026-10-22T10:30:20.000Z", from: { kind: "agent" }, text: "None of the 24 commits touch the store." },
    { kind: "message", id: "m3", at: "2026-10-22T10:30:40.000Z", from: { kind: "agent" }, text: "I'll merge main right before the next push rather than now. That push waits on your ship choice, and main keeps moving until then, so an earlier merge would just need repeating." },
    { kind: "message", id: "y1", at: "2026-10-22T10:35:00.000Z", from: { kind: "you" }, text: "Merge it now, the push is close." },
    { kind: "message", id: "m3b", at: "2026-10-22T10:35:20.000Z", from: { kind: "agent" }, text: "Merging main now." },
    { kind: "message", id: "m4", at: "2026-10-22T10:45:00.000Z", from: { kind: "agent" }, text: "The store tests pass in 0.6 seconds." },
    { kind: "message", id: "n2", at: "2026-10-22T11:00:00.000Z", from: FLEET, text: "slot-2 was cleaned. The files outside the branch are gone." },
    { kind: "tool", id: "t2", at: "2026-10-22T11:00:20.000Z", text: "Bash git status" },
    { kind: "message", id: "m5", at: "2026-10-22T11:00:40.000Z", from: { kind: "agent" }, text: "The branch is intact. Nothing to restore." },
    { kind: "message", id: "m6", at: "2026-10-22T11:10:00.000Z", from: { kind: "agent" }, text: "Ready to push the branch." },
    { kind: "message", id: "n4", at: "2026-10-22T11:12:00.000Z", from: FLEET, text: "#2734 merged into main. finish_up releases your slots and the branches already in main." },
    { kind: "message", id: "m8", at: "2026-10-22T11:12:20.000Z", from: { kind: "agent" }, text: "Releasing the slots now." },
    { kind: "tool", id: "t3", at: "2026-10-22T11:12:40.000Z", text: "armada waiting_for" },
    { kind: "message", id: "m9", at: "2026-10-22T11:12:50.000Z", from: { kind: "agent" }, text: "#2734 is merged. The only open item is in Needs you: run scripts/preview --restart." },
    { kind: "message", id: "n5", at: "2026-10-22T11:13:00.000Z", from: FLEET, text: "slot-5's branch armada/session-977003a3 merged. Lease it again before your next edit." },
    { kind: "message", id: "m10", at: "2026-10-22T11:13:20.000Z", from: { kind: "agent" }, text: "No edits are waiting on that slot." },
    { kind: "message", id: "n3", at: "2026-10-22T11:15:00.000Z", from: FLEET, text: "main moved under your branch: 3 files overlap with your changes." },
    { kind: "message", id: "m7", at: "2026-10-22T11:19:00.000Z", from: { kind: "agent" }, text: "The overlap is in the store's clock module. I need your pick before I touch it." },
    { kind: "ask", id: "ask-fold-q1", at: "2026-10-22T11:20:00.000Z", ask: ASKED, state: "waiting" },
  ],
});

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const served = new FakeSessionsFleet(records(), threads()).scenario(board);
  return {
    ...served,
    name: "fleet-turn-fold",
    says: "A Job's thread where Fleet's notices and the agent's replies to them fold into one dim row, and a turn that asks the owner stays open",
    behaves: (handle) => new FakeSessionsFleet(records(), threads()).scenario(board).behaves!(handle),
  };
}

export const s318FleetTurnFold: Scenario = build();
