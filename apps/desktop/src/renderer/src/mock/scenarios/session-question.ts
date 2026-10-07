// Two Sessions whose agent has asked the person something through its question tool: one with two
// questions (a single choice with option descriptions, and a multi-select) to answer, one with a
// single question to skip. Over the `sessions` scenario. The walk `session-question` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const asking: Session[] = [
  {
    id: "s11",
    title: "Order the lunch",
    turn: { state: "idle" },
    lastTurn: "14:22",
    lastTurnAt: minutesAgo(2),
    rows: [
      { id: "q1", at: "14:21:50", kind: "message", from: { kind: "you" }, text: "Write up the lunch order for Friday." },
      { id: "q2", at: "14:22:01", kind: "message", from: { kind: "agent" }, text: "I need two things before I write it." },
    ],
    attachments: [],
    asked: {
      command: "Which size?",
      call: "q-11",
      offers: ["allow_once", "refuse"],
      questions: [
        {
          question: "Which size?",
          header: "Size",
          multi_select: false,
          options: [
            { label: "Small", description: "One portion each" },
            { label: "Large", description: "Enough to share" },
          ],
        },
        {
          question: "Which toppings?",
          header: "Toppings",
          multi_select: true,
          options: [
            { label: "Cheese", description: "Melted on top" },
            { label: "Ham", description: "Sliced thin" },
            { label: "Olives", description: "Black, from a jar" },
          ],
        },
      ],
    },
  },
  {
    id: "s12",
    title: "Name the branch",
    turn: { state: "idle" },
    lastTurn: "14:18",
    lastTurnAt: minutesAgo(6),
    rows: [{ id: "n1", at: "14:18:04", kind: "message", from: { kind: "agent" }, text: "The branch needs a name." }],
    attachments: [],
    asked: {
      command: "Which prefix?",
      call: "q-12",
      offers: ["allow_once", "refuse"],
      questions: [
        {
          question: "Which prefix?",
          header: "Prefix",
          multi_select: false,
          options: [
            { label: "fix", description: "A bug fix" },
            { label: "feat", description: "New behaviour" },
          ],
        },
      ],
    },
  },
];

export const s204SessionQuestion: Scenario = {
  ...s200Sessions,
  name: "session-question",
  says: "A Session whose agent asks questions: answered with Other, and skipped",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, asking) },
};
