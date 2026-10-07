// What a Session's ledger draws: only the sections that hold a row, a small picture when it holds
// nothing, and the Artifacts a Session made. Over the `sessions` scenario with three more Sessions
// open beside its own: one holding a branch and a pull request, a brand-new one, and one that
// published a page, wrote a file and made a Claude Docs document. The walk `session-ledger` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;

const more: Session[] = [
  {
    id: "s8",
    terminal: true,
    title: "Pin the store clock",
    turn: idle,
    lastTurn: "14:20",
    rows: [],
    attachments: [
      { kind: "branch", name: "fix/pin-store-clock", slot: 2 },
      {
        kind: "pull_request",
        number: 1861,
        title: "Pin the store clock",
        branch: "fix/pin-store-clock",
        address: "https://example.com/pull/1861",
        checks: { state: "passed" },
        state: "open",
        auto: false,
      },
    ],
  },
  { id: "s9", title: "Store clock spike", turn: idle, lastTurn: "14:31", rows: [], attachments: [] },
  {
    id: "s10",
    terminal: true,
    title: "Write up the store clock",
    turn: idle,
    lastTurn: "14:28",
    rows: [],
    attachments: [
      { kind: "artifact", form: "page", id: "https://claude.ai/artifact/clock-findings", title: "Store clock findings" },
      { kind: "artifact", form: "file", id: "/Users/user/armada/docs/spikes/store-clock.md", title: "store-clock.md" },
      { kind: "artifact", form: "doc", id: "https://claude.ai/artifact/clock-writeup", title: "Store clock write-up" },
    ],
  },
];

export const s202SessionLedger: Scenario = {
  ...s200Sessions,
  name: "session-ledger",
  says: "A Session's ledger with some sections, with none, and with its Artifacts",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
