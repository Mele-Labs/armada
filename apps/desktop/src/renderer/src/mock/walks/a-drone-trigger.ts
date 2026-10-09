// A saved Trigger runs a Drone with a prompt (8 Oct 2026): the Trigger editor's Runs select offers Drone,
// which swaps the Command or Skill field for a Prompt, and drops Self repair, since a Drone already fixes
// its own failures. Saved, it fires like a Skill: a Job holds its run on a Drone branch of its own, and
// the Drone's fix asks where it goes.

import { button, dialog, inside, region, role, walk } from "../walk";

const NEW = dialog("New trigger");
const RUN = region("This Job's run");

export const aDroneTrigger = walk("real/job-2-drone-trigger", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { press: role("button", /^Handoff/), say: "The delivering step" },
  { press: button("Add trigger on PR opened"), say: "Add a Trigger on PR opened" },
  { type: "drone", into: inside(NEW, role("combobox", "Runs")), say: "Runs: a Drone" },
  { look: inside(NEW, role("textbox", "Prompt")), say: "A Prompt in place of the Command" },
  { type: "Add a changelog line", into: inside(NEW, role("textbox", "Prompt")), say: "What the Drone is asked to do" },
  { look: inside(NEW, role("group", "If it fails")), say: "If it fails: Block the Job, with no Self repair" },
  { press: inside(NEW, button("Save", { exact: true })), say: "Save" },
  { press: button("Cockpit", { exact: true }), say: "Back to the Board" },
  { press: button(/^Open Job/), say: "A Job it fired for" },
  { look: inside(RUN, role("img", "Drone branch")), say: "On a Job it fired for, a branch of its own" },
  { look: inside(RUN, role("group", "Where the fix goes")), say: "The Drone committed: its branch holds and asks" },
  { press: inside(RUN, button("New PR")), say: "New PR" },
  { look: inside(RUN, role("img", "New PR")), say: "The fix ends in a PR of its own" },
]);
