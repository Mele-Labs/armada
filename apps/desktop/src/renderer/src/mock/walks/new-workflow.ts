// A workflow made from nothing, and one a Job proposed. Over `new-workflow`, a mock with nothing written.
// Keys: N opens the form from the list, Tab moves through name, definition, scope and Save, Enter in the name saves, Escape cancels.

import { button, deckRow, inside, place, region, role, text, toSystemPage, walk } from "../walk";

const form = role("form", "New workflow");
const needs = region("Needs you");
const card = role("article", "Workflow: release-notes", { exact: true });

const newWorkflow = walk("new-workflow", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: button(/^New workflow/), say: "New workflow opens a blank definition" },
  { type: "release-notes", into: inside(form, role("textbox", "Name")), say: "A name" },
  { press: inside(form, role("radio", "This repository")), say: "Kept for this repository only; the other choice is every repository on this Mac" },
  { look: inside(form, button("Save", { exact: true })), say: "Save and Cancel" },
  { press: place("Cockpit"), say: "Back to the Cockpit" },
  { press: deckRow("Write release notes"), say: "A Job that proposed a workflow" },
  { look: inside(card, text("Every repository on this Mac")), say: "The card names the workflow, its scope and its definition. Approve or Dismiss" },
  { press: inside(needs, button(/^Needs you/)), say: "The same proposal is in Needs you" },
  { look: inside(needs, text("Dismiss")), say: "Name, scope and definition, with Approve and Dismiss" },
]);

export { newWorkflow as "new-workflow" };
