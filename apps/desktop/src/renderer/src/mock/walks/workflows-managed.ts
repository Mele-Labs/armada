// Workflows, managed (27 Oct 2026): System › Kit › Workflows as a list grouped by where each comes from, and
// the picked one as its steps, with the words the agent is told editable where they stand. Armada's rows
// are read-only, a repository's copy compares with Armada's, and a change of shape is typed to an agent whose
// diff waits on Approve. Jobs Fleet nudged at a step are listed under it, each with Open Job and Not a drift, and
// a suggested change for the step's words waits on Approve. A mock, no Fleet behind it.
// The decision of 27 Oct, a feature Job walks before it tests.

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
  { look: inside(step("Walk"), role("group", /^Cockpit timeline rail/)), say: "Drifted here: a Job Fleet nudged at Walk, with Open Job and Not a drift" },
  { look: inside(step("Walk"), role("group", "Suggested change")), say: "A suggested change drawn from the two Jobs, new words marked" },
  { hover: inside(step("Walk"), role("img", "In the suggested change")), say: "Marks the Jobs the change came from" },
  { press: inside(inside(step("Walk"), role("group", "Suggested change")), button("Approve", { exact: true })), say: "Approve applies the words" },
  { look: inside(step("Walk"), role("textbox", "Words")), say: "Walk's words, with the new line" },
  { press: inside(step("Approve plan"), button("Suggest a change")), say: "Suggest a change, asked for at Approve plan" },
  { look: inside(step("Approve plan"), role("group", "Suggested change")), say: "A suggested change for Approve plan" },
  { press: inside(step("Approve plan"), button("Not a drift")), say: "Not a drift takes the row, and the suggestion with it" },
  { press: inside(step("Propose"), role("textbox", "Words")), say: "Propose's words, edited in place" },
  {
    type: "Show what you took the request to mean and a first idea as a sketch with show_sketch, or a rough walk with show_window where the idea is about how something looks, with at most two lines of text beside it. No paragraphs, no lists of options in prose. Read only what that needs. Name what you could not tell. Then stop and wait for the person's reply.",
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
