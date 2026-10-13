// Two Sessions with a question nobody answered in time. One agent asked through its question tool and
// the call lapsed: the thread keeps a line for it and the questions stay under the thread as the deck.
// The other put a question with choices on its waiting list, and the Needs you band draws it as the same
// deck. Over the `sessions` scenario. The walk `lapsedQuestion` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const lapsed: Session = {
  id: "s50",
  title: "Book the table",
  turn: { state: "idle" },
  lastTurn: "19:02",
  lastTurnAt: minutesAgo(20),
  rows: [
    { id: "l1", at: "18:40:10", kind: "message", from: { kind: "you" }, text: "Book a table for the team dinner." },
    { id: "l2", at: "18:41:02", kind: "message", from: { kind: "agent" }, text: "I need the night and the place before I can book." },
    { id: "l3", at: "18:55:03", kind: "tool", text: "AskUserQuestion: no answer" },
  ],
  attachments: [],
  lapsed: {
    id: "l3",
    questions: [
      {
        question: "Which night?",
        header: "Night",
        multi_select: false,
        options: [
          { label: "Thursday", description: "The quiet night" },
          { label: "Friday", description: "Busier, the kitchen stays open late" },
        ],
      },
    ],
  },
};

const options: Session = {
  id: "s51",
  title: "Tidy the fixtures",
  turn: { state: "idle" },
  lastTurn: "19:10",
  lastTurnAt: minutesAgo(4),
  rows: [
    { id: "o1", at: "19:05:30", kind: "message", from: { kind: "you" }, text: "Tidy the store fixtures." },
    { id: "o2", at: "19:10:12", kind: "message", from: { kind: "agent" }, text: "One fixture is unused. It is your call." },
  ],
  attachments: [],
  waitingFor: [
    {
      id: "o3",
      text: "Keep the old fixture or drop it?",
      since: minutesAgo(4),
      source: "agent",
      options: [
        { label: "Keep", description: "Leave the file as it is" },
        { label: "Drop", description: "Delete the file and the tests that read it" },
      ],
    },
  ],
};

export const s241LapsedQuestion: Scenario = {
  ...s200Sessions,
  name: "lapsed-question",
  says: "A question that lapsed, still answerable under the thread, and a waiting item with choices drawn as a deck",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [lapsed, options]) },
};
