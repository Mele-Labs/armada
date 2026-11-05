// The Operator panel's parts the walk `operatorFull` did not show, over its Jobs: what was raised with no Job to stand on, what a role at
// Suggest would do, a repository's own positions, a standing order confirmed from the box, and reports as a kind in Issues.
// `?take=jobs-row` draws a raise's pulse on a row of the Jobs list as well as the title row's button. For the walk `operatorPanel`.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import type { OperatorSeed } from "../../operator/model";
import { operatorFull } from "./operator";
import { reportedBugs } from "./reported-bugs";

const armada: RepositorySummary = {
  root: "/Users/user/code/armada",
  records_root: "/records/armada",
  manifest: {
    ...repository().manifest!,
    id: "armada",
    repository: "armada",
    path: "armada/armada.yml",
  },
};

const storefront: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: {
    ...repository().manifest!,
    id: "storefront",
    repository: "storefront",
    path: "storefront/armada.yml",
  },
};

const take =
  typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("take");

const full = operatorFull.draft!.operator!;
const OWNER = repository().manifest!.id;

const seed: OperatorSeed = {
  ...full,
  raisesPulse: take === "jobs-row" ? "row" : "button",
  // Two rows arose in storefront, the rest in the repository these Jobs stand in, so the picker has something to narrow.
  log: full.log.map((row) => ({
    ...row,
    repository: row.id === "log-late" ? "storefront" : OWNER,
  })),
  line: full.line.map((row) => ({
    ...row,
    repository: row.id === "line-key" ? "storefront" : OWNER,
  })),
  raises: [
    {
      id: "raise-main",
      suggests: false,
      repository: "armada",
      manifest: OWNER,
      item: "main is red: desktop_unit failed",
      does: "started",
      read: "No live Job is fixing it",
      ways: [
        "Dispatch",
        "Skip",
      ],
    },
    {
      id: "raise-review",
      suggests: false,
      repository: "storefront",
      manifest: "storefront",
      item: "Pull request 412 asks for your review",
      does: "started",
      read: "Changes the checkout you walked on Monday",
      ways: [
        "Agent review",
        "Skip",
      ],
    },
    {
      id: "raise-report",
      suggests: false,
      repository: "armada",
      manifest: OWNER,
      item: "Report: Sheet stays open, but its Job ended",
      does: "started",
      read: "It holds a local path, so it cannot be filed as written",
      ways: [
        "Dispatch",
        "Drop",
      ],
    },
    {
      id: "raise-issue",
      suggests: true,
      repository: "armada",
      manifest: OWNER,
      item: "Would start a Job on: Quick dispatch loses its repository chip",
      does: "started",
      read: "New issue. No live Job plans it",
      ways: [
        "Do it",
        "Skip",
      ],
    },
    {
      id: "raise-later",
      suggests: true,
      repository: "armada",
      manifest: OWNER,
      item: "Would file: Record drops a step under a second",
      does: "filed",
      read: "A Later item from Time each build step. No open issue matches",
      ways: [
        "Do it",
        "Skip",
      ],
    },
  ],
  served: [
    "armada",
    "billing",
    "storefront",
  ],
  repositories: [
    {
      name: "storefront",
      positions: {
        dispatch: "suggest",
        answer: "off",
      },
    },
  ],
  model: "Sonnet",
  models: [
    "Opus",
    "Sonnet",
    "Haiku",
  ],
  talk: [
    ...full.talk,
    {
      asked: "hold anything touching the cockpit",
      said: "That can stand as this order. It holds starts on those paths, and it goes when Cockpit Console ends.",
      order: {
        id: "order-cockpit",
        kind: "hold_paths",
        paths: "apps/desktop/src/renderer/src/cockpit",
        until: {
          job: "Cockpit Console",
          event: "ends",
        },
      },
    },
  ],
};

function build(): Scenario {
  const base = holding(
    "operator-panel",
    "The Operator panel with what was raised on no Job, Suggest's rows, a repository's positions, an order from the box and reports in Issues",
    [
      repository(),
      armada,
      storefront,
    ],
  );
  return {
    ...base,
    state: {
      ...base.state,
      repository: null,
    },
    draft: {
      ...reportedBugs.draft,
      ...operatorFull.draft,
      operator: seed,
    },
  };
}

export const operatorPanel: Scenario = build();
