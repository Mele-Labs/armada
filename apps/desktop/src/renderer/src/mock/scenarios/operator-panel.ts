// The Operator's parts beyond `operatorFull`, over its Jobs: what was raised with no Job to stand on, the repositories it may offer a Job
// in, and reports as a kind in Issues.

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

const seed: OperatorSeed = {
  ...full,
  raisesPulse: take === "jobs-row" ? "row" : "button",
  raises: [
    {
      id: "raise-main",
      item: "main is red: desktop_unit failed",
    },
    {
      id: "raise-review",
      item: "Pull request 412 asks for your review",
    },
    {
      id: "raise-report",
      item: "Report: Sheet stays open, but its Job ended",
    },
    {
      id: "raise-issue",
      item: "Would start a Job on: Quick dispatch loses its repository chip",
    },
    {
      id: "raise-later",
      item: "Would file: Record drops a step under a second",
    },
  ],
  served: [
    "armada",
    "billing",
    "storefront",
  ],
};

function build(): Scenario {
  const base = holding(
    "operator-panel",
    "The Operator page with what was raised on no Job, the repositories a Job may start in, and reports in Issues",
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
