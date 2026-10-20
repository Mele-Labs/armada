// One Session whose agent asks four questions at once, to see what the answered ones look like as they pile up. Over the `sessions`
// scenario. The walk `session-question-many` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pick = (question: string, header: string, labels: string[]) => ({
  question,
  header,
  multi_select: false,
  options: labels.map((label) => ({ label, description: "" })),
});

const asking: Session[] = [
  {
    id: "s21",
    title: "Plan the offsite",
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(2),
    rows: [{ id: "m1", at: "14:22:01", kind: "message", from: { kind: "agent" }, text: "Four things before I book it." }],
    attachments: [],
    asked: {
      command: "Which city?",
      call: "q-21",
      offers: ["allow_once", "refuse"],
      questions: [
        pick("Which city?", "City", ["Lisbon", "Berlin"]),
        pick("Which week?", "Week", ["Mar 3", "Mar 10"]),
        pick("Which venue?", "Venue", ["Loft", "Hotel"]),
        pick("Which catering?", "Catering", ["Buffet", "Plated"]),
      ],
    },
  },
];

export const s205SessionQuestionMany: Scenario = {
  ...s200Sessions,
  name: "session-question-many",
  says: "A Session whose agent asks four questions",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, asking) },
};
