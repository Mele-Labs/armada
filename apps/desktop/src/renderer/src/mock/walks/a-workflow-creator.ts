// The Workflow creator, a rail row of its own: the workflows Fleet resolved as
// rows marked by where each came from, the picked one drawn as a graph on the
// Job's canvas with a back edge wherever a step sends work back, and the step
// or the workflow's settings in the app's one overlay panel. A mock, no Fleet
// behind it.

import { button, card, inside, role, text, walk } from "../walk";

const FRAME = role("region", "Workflow definition");
const PANEL = role("dialog", "gather");

export const aWorkflowCreator = walk("every-state", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { hover: role("img", "A workflow file cannot run"), say: "The rail row is marked while a file cannot run" },
  { look: role("list", "Workflow files"), say: "Every file, a row each" },
  { hover: role("img", /review returns to fix/), say: "The workflow as dots, an arc for a step sending work back" },
  { hover: role("img", "Ships with Armada"), say: "Carried: ships with Armada" },
  { hover: role("img", "Your machine"), say: "Kit: your machine" },
  { hover: role("img", "This repository"), say: "Repository: this repo" },
  { hover: role("img", "Left out"), say: "Left out: cannot run, and why" },
  { press: button("hotfix, kit, left out"), say: "Left out, with its reason" },
  { look: role("region", "hotfix, left out"), say: "The file and the reason" },
  { press: button("bug, repository"), say: "A workflow is its graph" },
  { look: card("repro"), say: "A step is a node" },
  { look: text("up to 4 passes"), say: "A step sending work back to an earlier one" },
  { press: button("release_notes, kit"), say: "Another workflow" },
  { press: card("gather"), say: "A step opens in the overlay panel" },
  { look: PANEL, say: "Its fields, checks, Judge and gate" },
  { press: inside(PANEL, role("checkbox", "Judge")), say: "Judge ticked, no question" },
  { press: inside(PANEL, button("Close")), say: "Close" },
  { press: button("Save", { exact: true }), say: "Save" },
  { look: text("Not saved"), say: "Refused, with the rule it broke" },
  { look: card("gather"), say: "The step it is about carries the reason" },
  { press: card("gather"), say: "Open the step" },
  { type: "Does the note cover every merged pull request?", into: role("textbox", "Judge question"), say: "The Judge question" },
  { press: inside(PANEL, button("Close")), say: "Close" },
  { press: button("Settings", { exact: true }), say: "Workflow settings, in the same panel" },
  { look: role("combobox", "Scope"), say: "Scope: Kit or a Manifest, from a dropdown" },
  { press: button("Close"), say: "Close" },
  { press: button("Save", { exact: true }), say: "Save" },
  { look: text(/^Saved$/), say: "Saved" },
  { press: button("Discuss with Helm"), say: "The draft goes to Helm" },
  { look: role("img", "Handed to Helm"), say: "Handed over" },
  { press: button("New workflow"), say: "A new definition, in this Manifest" },
  { press: button("Save", { exact: true }), say: "Save with nothing filled in" },
  { look: FRAME, say: "Refused: the ids are empty" },
]);
