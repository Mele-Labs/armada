// The Operator's Queue with work in it: rows he drags to set what it takes next, and an item waiting on him that he works through in Talk.
// Time moves the Operator on, a row at a time. For the walk `operatorQueue`.

import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { alsoOnTimePassing } from "../time-passes";
import { operatorPanel } from "./operator-panel";

const full = operatorPanel.draft!.operator!;

const SEND_RUNNING = {
  press: "Send: Running bays first",
  result: "Sent: Keep slot order, with running bays first",
  read: "I would send: Keep slot order, with running bays first",
};

function build(): Scenario {
  const base = holding(
    "operator-queue",
    "The Operator's Queue: rows he drags to set what it takes next, and an item waiting on him that he works through in Talk",
    [
      repository(),
    ],
  );
  return {
    ...base,
    draft: {
      ...operatorPanel.draft,
      operator: {
        ...full,
        // The list, as it was walked before the board.
        take: "list",
        moves: alsoOnTimePassing,
        talk: [
          {
            asked: "doctor checks",
            said: "Doctor lists new checks is next. It was waiting on Store Reports Its Size, so I told that Job to push its store change first.",
            puts: {
              turn: "turn-doctor",
              line: "You asked for this first. I told Store Reports Its Size to push first",
            },
          },
          ...full.talk,
        ],
        turns: [
          {
            id: "turn-timings",
            text: "Time each build step asks where to keep the timings",
            repository: "armada",
            finish: "Answered: Keep them in the store",
          },
          {
            id: "turn-legend",
            text: "Rework the Map legend asks whether to keep the old key",
            repository: "armada",
            finish: "Answered: Drop the old key",
          },
          {
            id: "turn-doctor",
            text: "Doctor lists new checks",
            start: {
              repository: "armada",
              waits: "Store Reports Its Size finishes",
            },
            finish: "Started",
          },
          {
            id: "turn-watch",
            text: "A watch stayed live after its pull request merged",
            about: "You dispatched it",
            start: {
              repository: "armada",
            },
            finish: "Started",
          },
        ],
        raises: [
          {
            id: "raise-console",
            tag: "Question",
            suggests: true,
            repository: "armada",
            item: "Cockpit Console asks which way to draw the bays",
            read: "I would tell it to keep the current order",
            ways: [
              "Send: Keep the order",
            ],
            results: {
              "Send: Keep the order": "Sent: Keep the current order",
            },
            does: "delegated",
            job: {
              session: "op-console",
              title: "Cockpit Console",
            },
            talk: {
              first:
                "It has two ways to draw them. Its plan says bays keep their slot order, so I would tell it to keep the current order.",
              replies: [
                {
                  asked: "running",
                  said: "Then I would send it this: Keep slot order, with running bays first. The press on the card now says that.",
                  send: SEND_RUNNING,
                },
              ],
            },
          },
          {
            id: "raise-main",
            tag: "Main is red",
            suggests: false,
            repository: "armada",
            item: "desktop_unit failed on main",
            read: "No Job is fixing it",
            ways: [
              "Start a fix",
              "Ignore",
            ],
            results: {
              "Start a fix": "Started: Fix desktop_unit",
              Ignore: "Ignored",
            },
            does: "started",
            talk: {
              first:
                "desktop_unit failed on main. No live Job touches it, so I would start a Job to fix it.",
              replies: [],
            },
          },
          {
            id: "raise-review",
            tag: "Review",
            suggests: false,
            repository: "storefront",
            item: "Pull request 412 asks for your review",
            read: "Changes the checkout you walked on Monday",
            ways: [
              "Agent review",
              "Ignore",
            ],
            results: {
              "Agent review": "Started: Agent review",
              Ignore: "Ignored",
            },
            does: "started",
            talk: {
              first:
                "It changes the checkout you walked on Monday, so I brought it to you. An agent can review it first if you want.",
              replies: [],
            },
          },
          {
            id: "raise-file",
            tag: "New issue",
            suggests: true,
            repository: "armada",
            item: "Quick dispatch loses its repository chip",
            read: "No open issue covers it",
            ways: [
              "File it",
              "Don't file",
            ],
            results: {
              "File it": "Filed: armada-k2p",
              "Don't file": "Not filed",
            },
            queues: [
              "File it",
            ],
            does: "filed",
            talk: {
              first: "A Job noticed this, and no open issue covers it. I would file it.",
              replies: [],
            },
          },
        ],
      },
    },
  };
}

export const operatorQueue: Scenario = build();
