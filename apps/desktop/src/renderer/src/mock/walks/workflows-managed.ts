// Workflows, managed (27 Oct 2026): System › Kit › Workflows as a list grouped by where each comes from, and
// the picked one as its steps, with the words the agent is told editable where they stand. Armada's rows
// are read-only, a repository's copy compares with Armada's, and a change of shape is typed to an agent whose
// diff waits on Approve. A mock, no Fleet behind it.
// `.claude/decisions/2026-10-27-a-feature-job-walks-before-it-tests.md`.

import { button, inside, role, text, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const ARMADA = group("Armada");
const REPOSITORY = group("armada repository");
const YOURS = group("Yours");
const step = (name: string) => group(name);

export const workflowsManaged = walk("workflows", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: ARMADA, say: "Armada's: read-only" },
  { look: REPOSITORY, say: "This repository's copies" },
  { look: YOURS, say: "Yours" },
  { hover: role("img", "This repository's copy replaces Armada's"), say: "This repository's feature replaces Armada's" },
  { press: inside(REPOSITORY, button("feature", { exact: true })), say: "The repository's feature" },
  { look: inside(step("Walk"), text("you approve the walk")), say: "Walk ends when you approve the walk" },
  { look: inside(step("Walk"), role("button", /Cockpit timeline rail/)), say: "Drifted here: a Job Fleet nudged at Walk" },
  { press: inside(step("Propose"), role("textbox", "Words")), say: "Propose's words, edited in place" },
  {
    type: "Say in a few lines what you took the request to mean and a first idea. Read only what that needs. Name what you could not tell. Then stop and wait for the person's reply.",
    into: inside(step("Propose"), role("textbox", "Words")),
    say: "A sentence added",
  },
  { press: inside(step("Propose"), button("Save", { exact: true })), say: "Save" },
  { press: button("Compare with Armada's"), say: "Compare with Armada's" },
  { look: inside(step("Propose"), role("group", "Compared with Armada's")), say: "The changed step, Armada's words beside this copy's" },
  { type: "split Refine into Wire and Tests", into: role("textbox", "Tell an agent what to change"), say: "A change of shape, typed to an agent" },
  { press: button("Send to agent"), say: "Send" },
  { look: role("region", "Proposed change"), say: "Refine removed, Wire and Tests added" },
  { press: inside(role("region", "Proposed change"), button("Approve", { exact: true })), say: "Approve applies it" },
  { look: step("Wire"), say: "Wire is a step" },
  { look: step("Tests"), say: "Tests is a step" },
  { press: inside(ARMADA, button("bug", { exact: true })), say: "Armada's bug" },
  { press: button("Make my copy"), say: "Make my copy" },
  { look: inside(YOURS, button("bug", { exact: true })), say: "Bug, under Yours" },
]);
