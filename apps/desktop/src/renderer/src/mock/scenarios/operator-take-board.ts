// The Operator's Queue drawn as a board of lanes, for the walk `operatorTakeBoard`. The same moment as `operator-queue`.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { BASE, operatorJobs, operatorSessions, question } from "./operator";
import { operatorQueue } from "./operator-queue";

// The Job on the Now card, which opens over the Operator like every card's Job.
const timings: Session = {
  ...BASE,
  id: "op-timings",
  title: "Time each build step",
  turn: {
    state: "working",
  },
  rows: [
    {
      id: "op-12-u",
      at: "09:30:00",
      kind: "message",
      from: {
        kind: "you",
      },
      text: "Time each step of a build, so the Record can show where the minutes go.",
    },
    {
      id: "op-12-a",
      at: "09:52:00",
      kind: "message",
      from: {
        kind: "agent",
      },
      text: "The step timings are measured. One choice is left before the Record draws them.",
    },
  ],
  asked: question("op-q-12", "Where should it keep the timings?", "Timings", [
    [
      "In the store",
      "Beside the run it timed, read by the Record",
    ],
    [
      "Beside each run's log",
      "A file next to the log, read by nothing else",
    ],
  ]),
};

export const operatorTakeBoard: Scenario = {
  ...operatorQueue,
  name: "operator-take-board",
  says: "The Operator's Queue as a board: Up next, Operator on it, Needs you and Done, with cards that move between lanes",
  draft: {
    ...operatorQueue.draft,
    // The Job merged into another reads as Time each build step too: the board's card is the live one.
    sessions: () =>
      operatorSessions([
        ...operatorJobs.filter((one) => one.id !== "op-step-times"),
        timings,
      ]),
    operator: {
      ...operatorQueue.draft!.operator!,
      take: "board",
      // As the Fleet-backed Operator does: Discuss brings an item up at once.
      discussesAtOnce: true,
      // Each card names the Job it is about, with what it asks under, as Fleet's board and a Job's Needs you items arrive.
      turns: operatorQueue.draft!.operator!.turns?.map((one) =>
        one.id === "turn-timings"
          ? {
              ...one,
              text: "Where should it keep the timings?",
              job: {
                session: "op-timings",
                title: "Time each build step",
              },
            }
          : one.id === "turn-legend"
            ? {
                ...one,
                text: "Should it keep the old key?",
                job: {
                  session: "op-legend",
                  title: "Rework the Map legend",
                },
              }
            : one,
      ),
      // The Job's question with its own options, the Operator's pick marked, as a Fleet Job's item arrives.
      raises: operatorQueue.draft!.operator!.raises?.flatMap((one) =>
        one.id === "raise-console"
          ? [
              {
                ...one,
                item: "Cockpit Console",
                read: "Which way should it draw the bays?",
                ways: [
                  "Keep the order",
                  "Sort by state",
                ],
                recommends: "Keep the order",
                results: {
                  "Keep the order": "Answered: Keep the order",
                  "Sort by state": "Answered: Sort by state",
                },
              },
              {
                id: "raise-home",
                tag: "Question",
                suggests: true,
                repository: "armada",
                item: "Home Draws Desktop States",
                read: "Should a desktop's states match the Cockpit's colours?",
                ways: [
                  "Match the Cockpit",
                  "Keep its own",
                ],
                recommends: "Match the Cockpit",
                results: {
                  "Match the Cockpit": "Answered: Match the Cockpit",
                  "Keep its own": "Answered: Keep its own",
                },
                does: "answered",
                job: {
                  session: "op-home",
                  title: "Home Draws Desktop States",
                },
              },
            ]
          : [
              one,
            ],
      ),
    },
  },
};
