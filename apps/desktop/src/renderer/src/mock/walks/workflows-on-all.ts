// Workflows with All repositories picked (27 Oct 2026, lanes 31 Oct): every repository's copies in a group of its own, captioned
// by the repository they came from, between Armada's and Yours. A copy is drawn over the lanes of its own repository, and Armada's
// over the first repository's. A mock, no Fleet behind it.

import { button, inside, region, role, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Workflow", { exact: true });
const RUNS_IN = group("Runs in");
const SECOND = group("ledger repository");

export const workflowsOnAll = walk("workflows-on-all", [
  ...toSystemPage("Workflows", "Workflows, from System, with all repositories"),
  { look: LANES, say: "Armada's feature over the lanes, read for the first repository" },
  { press: inside(SECOND, button("bug", { exact: true })), say: "The second repository's bug" },
  { look: inside(RUNS_IN, role("tab", "ledger", { exact: true })), say: "Its lanes are ledger's" },
  { press: inside(LANES, button("Step: Fix", { exact: true })), say: "Fix, picked on the lanes" },
  { look: role("textbox", "Words", { exact: true }), say: "Fix's words, edited where they stand" },
]);
