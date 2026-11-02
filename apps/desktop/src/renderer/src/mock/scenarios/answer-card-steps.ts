// A Job whose agent asks the owner to send back three command outputs: one waiting item with two choices and
// the steps to follow, so its card draws the steps under the question and above the choices.
// The walk `answerCardSteps` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s22",
  title: "Trace the missing session id",
  turn: {
    state: "idle",
  },
  lastTurnAt: ago(2),
  rows: [
    {
      id: "c1",
      at: "09:12:40",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Find out why the session id is empty in a fresh Job.",
    },
    {
      id: "c2",
      at: "09:20:43",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "I cannot reproduce it from here. The engineer who saw it can send what a fresh Job prints.",
    },
  ],
  attachments: [],
  waitingFor: [
    {
      id: "c-outputs",
      text: "Ask the other engineer to send back three command outputs",
      since: ago(3),
      source: "agent",
      act: {
        kind: "answer",
        target: "",
      },
      options: [
        {
          label: "Sent",
        },
        {
          label: "Not needed",
        },
      ],
      steps: [
        "Update to a build with #3197",
        "Start a Job in the repository that failed before, and let it work for two turns",
        "In that Job's terminal, run `echo $ARMADA_SESSION`, `armada job status` and `armada report`",
        "Paste all three outputs back here",
      ],
    },
  ],
};

function build(): Scenario {
  const base = holding(
    "answer-card-steps",
    "A Job waiting on a question with two choices and four steps, one of them naming commands to run",
  );
  return {
    ...base,
    state: {
      ...base.state,
      repository: repository().root,
    },
    draft: {
      sessions: () =>
        sessionsStore([
          SESSION,
        ]),
    },
  };
}

export const s344AnswerCardSteps: Scenario = build();
