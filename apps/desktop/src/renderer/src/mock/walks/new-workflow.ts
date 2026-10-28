// Making a workflow: New workflow on the Workflows page opens the Dispatch sheet for a Job, which proposes it as a card in its thread
// and an item in Needs you; Approve lists it under its scope. Over `new-workflow`, a mock with the Job scripted.
// Keys: N opens the sheet from the Workflows page, Enter starts, Esc closes, 1 and 2 answer in Needs you.

import { button, dialog, inside, region, role, toSystemPage, walk } from "../walk";

const sheet = dialog("Dispatch");
const needs = region("Needs you");

const newWorkflow = walk("new-workflow", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: button(/^New workflow/), say: "New workflow opens the sheet" },
  { type: "Release notes for a tag", into: inside(sheet, role("textbox", "Request")), say: "What it should do" },
  { press: inside(sheet, role("radio", "This repository")), say: "Where it is kept" },
  { press: inside(sheet, button("Start", { exact: true })), say: "Start begins a Job, which opens in the Cockpit" },
  { press: inside(needs, button(/^Needs you/)), say: "The proposed workflow is in Needs you, and as a card in the thread" },
  { press: inside(needs, role("radio", /Approve/)), say: "Approve" },
  { press: inside(needs, button("Answer", { exact: true })), say: "Answer sends it" },
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: role("button", "release-notes", { exact: true }), say: "release-notes is listed" },
]);

export { newWorkflow as "new-workflow" };
