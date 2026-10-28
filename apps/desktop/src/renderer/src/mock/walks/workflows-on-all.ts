// Workflows with All repositories picked (27 Oct 2026): every repository's copies in a group of its own, captioned
// by the repository they came from, between Armada's and Yours. A copy opens and saves under its own repository.
// A mock, no Fleet behind it.

import { button, inside, role, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const ARMADA = group("Armada");
const FIRST = group("armada repository");
const SECOND = group("ledger repository");
const YOURS = group("Yours");
const step = (name: string) => group(name);

export const workflowsOnAll = walk("workflows-on-all", [
  ...toSystemPage("Workflows", "Workflows, from System, with all repositories"),
  { look: ARMADA, say: "Armada's, once" },
  { look: FIRST, say: "The first repository's copies" },
  { look: SECOND, say: "The second repository's copies" },
  { look: YOURS, say: "Yours, once" },
  { press: inside(SECOND, button("bug", { exact: true })), say: "The second repository's bug" },
  { press: inside(step("Fix"), role("textbox", "Words")), say: "Fix's words" },
  {
    type: "Make the smallest change that addresses the cause. Add a line to the changelog. Write no tests yet.",
    into: inside(step("Fix"), role("textbox", "Words")),
    say: "Edited in place",
  },
  { press: inside(step("Fix"), button("Save", { exact: true })), say: "Save, under the second repository" },
  { look: inside(step("Fix"), role("textbox", "Words")), say: "Fix's words, as saved" },
]);
