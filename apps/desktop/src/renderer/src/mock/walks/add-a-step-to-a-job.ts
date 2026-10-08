// A step added to one Job from its workflow (7 Oct 2026): a `+` after the step the Job is on and
// after each step to come, offering a Script, a Skill or a Drone step. The mock Fleet answers as
// Fleet does: the row whole, `pending` until its moment, a Script run and passed, a Skill run on a
// Drone of its own. A step is filled in and added; Fleet has no edit for one, so what is still to do with it
// is take it off before it fires, and keep it for every Job.

import { button, card, dialog, inside, role, tab, text, walk } from "../walk";

const NEW = dialog("Script");
const ADDED = dialog("fmt");

export const addAStepToAJob = walk("proto/feature-running", [
  { press: tab("Workflow"), say: "The Job's workflow, on its canvas" },
  { hover: button("Add a step after Implement"), say: "A + after the step the Job is on, and after each step to come" },
  { press: button("Add a step after Implement"), say: "Add a step there" },
  { look: role("menu"), say: "A Script, a Skill or a Drone step, over the canvas" },
  { press: role("menuitem", "Script"), say: "A Script: a Command from armada.yml, which Fleet runs" },
  { look: NEW, say: "Its fields, for this Job only" },
  { type: "fmt", into: inside(NEW, role("combobox", "Script")), say: "A Command the repository declares" },
  { press: inside(NEW, role("switch", "Self repair")), say: "Self repair on, Block the Job off" },
  { look: inside(NEW, role("group", "If it fails")), say: "The same two switches a Trigger has" },
  { press: inside(NEW, button("Add", { exact: true })), say: "Add" },
  { look: role("button", /^fmt, Added to this Job only, pending/), say: "On the canvas, panned to, drawn apart from the workflow's own steps" },
  { hover: role("button", /^fmt, Added to this Job only/), say: "Dashed accent edge and the Trigger mark, with its tooltip" },
  { later: role("button", /^fmt, Added to this Job only, pending/), say: "Pending until its moment comes" },
  { later: role("button", /^fmt, Added to this Job only, running/), say: "It runs" },
  { look: role("button", /^fmt, Added to this Job only, passed/), say: "And passes" },
  { press: tab("Stacked"), say: "The stacked run offers the same +" },
  { press: button("Add a step after Review the change"), say: "A step after the one that opens the PR" },
  { press: role("menuitem", "Skill"), say: "A Skill" },
  { type: "qa-notes", into: inside(dialog("Skill"), role("textbox", "Skill")), say: "Its name" },
  { press: inside(dialog("Skill"), button("Add", { exact: true })), say: "Add" },
  { later: role("button", /^qa-notes, Added to this Job only, pending/), say: "Its moment comes" },
  { look: role("button", /^qa-notes, Added to this Job only, running/), say: "A Skill runs on a Drone of its own" },
  { press: tab("Canvas"), say: "Back to the canvas" },
  { press: card("fmt"), say: "Open the added step again" },
  { press: inside(ADDED, button("Keep for every Job")), say: "Keep it for every Job" },
  { look: inside(dialog("fmt"), role("combobox", "Set in")), say: "The Trigger editor, filled in: This machine or Repository" },
  { press: inside(dialog("fmt"), button("Keep", { exact: true })), say: "Keep" },
  { press: inside(dialog("fmt"), button("Close")), say: "Close" },
  { press: card("fmt"), say: "The added step says where it was kept" },
  { look: inside(ADDED, text("Kept: this machine")), say: "Kept for every Job, on this machine" },
]);
