// Making a workflow: New workflow on the Workflows page opens the Dispatch sheet for a Job, which proposes it as a card in its thread
// and an item in Needs you; Approve lists it under its scope. Over `new-workflow`, a mock with the Job scripted.
// Keys: N opens the sheet from the Workflows page, Enter starts, Esc closes, Approve answers in the Needs you sheet.

import { button, dialog, inside, role, toSystemPage, walk } from "../walk";
import { openNeeds } from "../sessions/walk-kit";

const sheet = dialog("Dispatch");
const needs = dialog(/^(?!Dispatch)/);

const newWorkflow = walk("new-workflow", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  {
    press: button(/^New workflow/),
    say: "New workflow opens the sheet",
  },
  {
    type: "Release notes for a tag",
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
    ...openNeeds,
    say: "The proposed workflow is in the Needs you sheet, and as a card in the thread",
  },
  {
    press: inside(needs, button(/^Approve/)),
    say: "Approve, on the draft in the sheet",
  },
  ...toSystemPage("Workflows", "Workflows, from System"),
  {
    look: role("button", "release-notes", {
      exact: true,
    }),
    say: "release-notes is listed",
  },
]);

export { newWorkflow as "new-workflow" };
