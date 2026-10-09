// A Session that waits on the person for four things, one of each act: a page to approve in a window,
// a pull request to approve, a command to run, and an open question Fleet knows of. Over the `sessions`
// scenario. The walk `session-waiting-on-you` plays it.

import { mockPage } from "@armada/jobs/fake";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const waiting: Session = {
  id: "s14",
  title: "Pin the store clock",
  turn: { state: "idle" },
  lastTurn: "14:40",
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "w1", at: "14:38:02", kind: "message", from: { kind: "you" }, text: "Pin the clock in the store tests." },
    { id: "w2", at: "14:40:11", kind: "message", from: { kind: "agent" }, text: "The change is up. I need a few things from you." },
  ],
  attachments: [
    { kind: "slot", slot: 3 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 3 },
    {
      kind: "pull_request",
      number: 1843,
      title: "Pin the store clock",
      branch: "fix/pin-store-clock",
      address: "https://example.com/pull/1843",
      checks: { state: "passed" },
      state: "open",
      auto: false,
    },
  ],
  asked: {
    command: "Which clock?",
    call: "q-14",
    offers: ["allow_once", "refuse"],
    questions: [
      {
        question: "Which clock?",
        header: "Clock",
        multi_select: false,
        options: [
          { label: "Fake", description: "A clock the test sets" },
          { label: "Frozen", description: "The real clock, stopped" },
        ],
      },
    ],
  },
  waitingFor: [
    { id: "a1", text: "Look at the findings page", since: minutesAgo(3), source: "agent", act: { kind: "walk", target: mockPage() } },
    { id: "a2", text: "Approve #1843", since: minutesAgo(3), source: "agent", act: { kind: "approve_pr", target: "1843" } },
    { id: "a3", text: "Run the store tests on your machine", since: minutesAgo(2), source: "agent", act: { kind: "run", target: "cargo test -p store" } },
    {
      id: "q1",
      text: "Which clock?",
      since: minutesAgo(3),
      source: "ask_card",
      act: { kind: "answer", target: "q-14" },
      options: [{ label: "Fake" }, { label: "Frozen" }],
    },
  ],
};

export const s206SessionWaitingOnYou: Scenario = {
  ...s200Sessions,
  name: "session-waiting-on-you",
  says: "A Session waiting on the person for a page, a pull request, a command and a question",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
