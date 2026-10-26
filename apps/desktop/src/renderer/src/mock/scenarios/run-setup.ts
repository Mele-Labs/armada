// The same Cockpit as `run-an-app`, with nothing installed in auto-ynab's checkout: Fleet drafts its setup from the repository's files, `npm ci`
// in `frontend` and a `.venv` from `requirements.txt` at the root, and runs it before the processes start. The walk `run-setup` plays it;
// `run-setup-failed` has `npm ci` drop its connection the first time, and the walk of that name plays it.

import type { RunSetupSpec } from "@armada/components";

import type { Scenario } from "../moment";
import type { RunsSeed } from "../runs";
import { runAnApp } from "./run-an-app";

const SETUP: readonly RunSetupSpec[] = [
  { id: "frontend-deps", name: "frontend packages", folder: "frontend", command: "npm ci" },
  { id: "venv", name: ".venv", folder: ".", command: "python3 -m venv .venv && .venv/bin/pip install -r requirements.txt" },
];

function build(name: string, says: string, failsOnce?: string): Scenario {
  const seed = runAnApp.draft?.runs as RunsSeed;
  const drafts = Object.fromEntries(Object.entries(seed.drafts).map(([root, draft]) => [root, { ...draft, setup: SETUP }]));
  return {
    ...runAnApp,
    name,
    says,
    draft: { ...runAnApp.draft, runs: { ...seed, drafts, ...(failsOnce === undefined ? {} : { failsOnce }) } },
  };
}

export const runSetup: Scenario = build("run-setup", "The Cockpit with auto-ynab, nothing installed in its checkout");
export const runSetupFailed: Scenario = build("run-setup-failed", "The Cockpit with auto-ynab, whose npm ci fails the first time it runs", "frontend-deps");
