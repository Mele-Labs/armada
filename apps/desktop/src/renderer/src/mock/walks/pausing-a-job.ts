// Pause and Resume. A running Job is paused through its confirm: it reads queued
// with the paused mark, and its slot frees. A Job at its review gate is paused
// and keeps Needs review, gaining the mark beside the badge. Approve pressed on
// it opens the Resume confirm instead of a refusal; Resume sends nothing but
// itself. From Cleanup, a Job parked at its gate is paused from its tile's panel.

import { putOff } from "../walk-steps";
import { button, dialog, inside, role, tab, text, walk } from "../walk";

const RUNNING = "Debounce the Job Board";
const GATE = "Retry a short read once";
const PARKED = "Port the settings selectors";

const MORE = (title: string) => button(`More for ${title}`);
const PAUSE_DIALOG = dialog("Pause this job?");
const RESUME_DIALOG = dialog("This job is paused");
const MARK = role("img", /^Paused/);
const TILE = (n: number) => inside(role("listitem", `slot-${n}`, { exact: true }), role("button", `slot-${n}`, { exact: true }));
const PANEL = (n: number) => dialog(`slot-${n}`);

export const pausingAJob = walk("pausing/jobs", [
  ...putOff(1),
  { press: tab("Active"), say: "A running Job, on Active" },
  { press: role("option", new RegExp(RUNNING)), say: "Picked, its acts beside it" },
  { press: MORE(RUNNING), say: "Pause is in the caret beside the verb" },
  { press: role("menuitem", "Pause", { exact: true }), say: "Pause asks first" },
  { look: inside(PAUSE_DIALOG, text("Stops the Drone and its processes")), say: "It stops the Drone and its processes" },
  { look: inside(PAUSE_DIALOG, text("Commits uncommitted files to branch armada/71-debounce as a WIP commit")), say: "It commits the uncommitted files to the branch as a WIP commit" },
  { look: inside(PAUSE_DIALOG, text("Releases slot-1")), say: "It releases the slot" },
  { look: inside(PAUSE_DIALOG, text("The step restarts on Resume")), say: "The step restarts on Resume" },
  { press: inside(PAUSE_DIALOG, button("Pause", { exact: true })), say: "Paused" },
  { hover: MARK, say: "The Job reads queued, and the mark says where its work is" },
  { press: role("button", "Worktree Slots", { exact: true }), say: "Its slot is back in the pool" },
  { look: inside(role("listitem", "slot-1", { exact: true }), role("img", "Free", { exact: true })), say: "slot-1 is free, and the Job's work is on its branch" },
  { press: role("button", "Cockpit", { exact: true }), say: "Back to the Dashboard" },
  { press: tab("Active"), say: "A Job at its review gate, on Active" },
  { press: role("option", new RegExp(GATE)), say: "Picked" },
  { press: MORE(GATE), say: "A Job at its gate: the same caret" },
  { press: role("menuitem", "Pause", { exact: true }), say: "Pause asks first" },
  { look: inside(PAUSE_DIALOG, text("Releases slot-2")), say: "A Job at a gate has no process, so it lists only the release" },
  { press: inside(PAUSE_DIALOG, button("Pause", { exact: true })), say: "Paused" },
  { press: button("Review", { exact: true }), say: "Open it" },
  { look: text("Needs review"), say: "It keeps Needs review, with the mark beside the badge" },
  { look: role("img", /^Paused/), say: "The mark stands beside the badge" },
  { press: button("Approve the work"), say: "Approve on a paused Job" },
  { press: button("Approve and drop them"), say: "It asks about the listed change first, as it does on any Job" },
  { look: inside(RESUME_DIALOG, text("Does not send the act you pressed")), say: "It offers Resume, and does not send the Approve" },
  { press: inside(RESUME_DIALOG, button("Resume", { exact: true })), say: "Resume only resumes" },
  { press: role("button", "Worktree Slots", { exact: true }), say: "Cleanup" },
  { press: TILE(3), say: "A Job parked at its gate holds slot-3" },
  { press: inside(PANEL(3), button("Pause", { exact: true })), say: "Pause is on the tile's panel" },
  { look: role("group", "Pause slot-3", { exact: true }), say: "It lists the release, and no process to stop" },
  { press: inside(role("group", "Pause slot-3", { exact: true }), button("Pause", { exact: true })), say: "Sent" },
  { look: inside(PANEL(3), role("img", /^Paused/)), say: `${PARKED} carries the mark, and its slot is free` },
]);
