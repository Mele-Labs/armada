// Making a workflow: New workflow on the Workflows page opens the Dispatch sheet for a Job, which proposes it as a card in its thread,
// a lane a step, and an item in Needs you; Approve lists it under its scope. Over `new-workflow`, a mock with the Job scripted.
// Keys: N opens the sheet from the Workflows page, Enter starts, Esc closes, 1 and 2 answer in Needs you.

import { button, dialog, inside, region, role, toSystemPage, walk } from "../walk";

const sheet = dialog("Dispatch");
const proposed = role("article", /^Workflow: annotations/);
const needs = region("Needs you");

const newWorkflow = walk("new-workflow", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  {
    press: button(/^New workflow/),
    say: "New workflow opens the sheet",
  },
  {
    type: "Work through the annotations I leave on the app",
    into: inside(sheet, role("textbox", "Request")),
    say: "What it should do",
  },
  {
    press: inside(sheet, role("radio", "This repository")),
    say: "Where it is kept",
  },
  {
    press: inside(
      sheet,
      button("Start", {
        exact: true,
      }),
    ),
    say: "Start begins a Job, which opens in the Cockpit",
  },
  {
    look: proposed,
    say: "The proposed workflow is a card in the thread: its name, what requests it is for, and a lane a step",
  },
  {
    look: inside(proposed, role("listitem", "Refresh preview and sign off")),
    say: "Each lane says what the step does and what ends it; this one asks how to deliver",
  },
  {
    press: inside(
      inside(proposed, role("listitem", "Refine and land")),
      button("What the agent is told"),
    ),
    say: "What the agent is told on a step opens under it",
  },
  {
    press: inside(needs, button(/^Needs you/)),
    say: "The proposed workflow is in Needs you too",
  },
  {
    press: inside(needs, role("radio", /Approve/)),
    say: "Approve",
  },
  {
    press: inside(
      needs,
      button("Answer", {
        exact: true,
      }),
    ),
    say: "Answer sends it",
  },
  ...toSystemPage("Workflows", "Workflows, from System"),
  {
    look: role("button", "annotations", {
      exact: true,
    }),
    say: "annotations is listed",
  },
]);

export { newWorkflow as "new-workflow" };
