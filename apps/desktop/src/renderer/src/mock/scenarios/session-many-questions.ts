// A Session whose agent asks four questions at once: more than the card has room for, so the card
// scrolls and the thread above keeps its share. Over the `sessions` scenario. The walk
// `questionDeck` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const choice = (question: string, header: string, labels: string[]) => ({
  question,
  header,
  multi_select: false,
  options: labels.map((label) => ({ label, description: `${label}, as it reads` })),
});

const asking: Session[] = [
  {
    id: "s13",
    title: "Plan the night",
    turn: { state: "idle" },
    lastTurn: "14:30",
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "m1", at: "14:29:40", kind: "message", from: { kind: "you" }, text: "Show me the sleep mode mock." },
      { id: "m2", at: "14:30:02", kind: "message", from: { kind: "agent" }, text: "The walk is open. Four things to settle." },
    ],
    attachments: [],
    asked: {
      command: "Should a destructive best answer be decided overnight?",
      call: "q-13",
      offers: ["allow_once", "refuse"],
      questions: [
        choice("Should a destructive best answer be decided overnight?", "Destructive", ["Hold for me", "Decide it"]),
        choice("Should the Morning review open when Sleep goes off?", "Open sheet", ["Only if non-empty", "Always"]),
        choice("Is the moon in the right place, left of Helm?", "Moon", ["Left of Helm", "Title bar right"]),
        choice("Are the five proposed icons right?", "Icons", ["Approve all", "Change some"]),
      ],
    },
  },
];

export const s205SessionManyQuestions: Scenario = {
  ...s200Sessions,
  name: "session-many-questions",
  says: "A Session whose agent asks four questions: one at a time",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, asking) },
};
