// A Session whose agent asks which checks to use, each option carrying the list it means as a
// preview. Over the `sessions` scenario. The walk `session-question-preview` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const asking: Session[] = [
  {
    id: "s13",
    title: "Pick the checks",
    turn: { state: "idle" },
    lastTurnAt: minutesAgo(1),
    rows: [
      { id: "p1", at: "14:29:40", kind: "message", from: { kind: "you" }, text: "Set up the checks for the three services." },
      { id: "p2", at: "14:30:01", kind: "message", from: { kind: "agent" }, text: "I have two lists in mind." },
    ],
    attachments: [],
    asked: {
      command: "Use these checks for ops, consumer and api-gateway?",
      call: "q-13",
      offers: ["allow_once", "refuse"],
      questions: [
        {
          question: "Use these checks for ops, consumer and api-gateway?",
          header: "Checks",
          multi_select: false,
          options: [
            {
              label: "The shared set",
              description: "The same three for each",
              preview: "ops          cargo test -p ops\nconsumer     cargo test -p consumer\napi-gateway  cargo test -p api-gateway",
            },
            {
              label: "Per service",
              description: "Each runs what it owns",
              preview: "ops          cargo test -p ops && cargo clippy -p ops\nconsumer     pnpm test --filter consumer\napi-gateway  go test ./...",
            },
            { label: "None yet", description: "Leave them for later" },
          ],
        },
      ],
    },
  },
];

export const s244SessionQuestionPreview: Scenario = {
  ...s200Sessions,
  name: "session-question-preview",
  says: "A Session whose question's options each show the checks they mean",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, asking) },
};
