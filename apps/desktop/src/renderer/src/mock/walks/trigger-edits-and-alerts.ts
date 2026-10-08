// What a person can still do about a Trigger's work (8 Oct 2026). A step added to a Job keeps its two
// switches live until it fires. A Drone step kept for every Job takes the next free name, and is
// never offered Replace on a clash. The Overview's lead lists the alerts, and one opens its Job at the
// Trigger, where a file of the held fix opens its diff against the Job's branch.

import { button, card, dialog, inside, role, tab, walk } from "../walk";

const NEW = dialog("Script");
const ADDED = dialog("fmt");
const DRONE = dialog("Drone step");
const ALERTS = role("list", "Alerts");
const KEEP = dialog("New trigger");
const FIX = dialog("Fix diff");

export const triggerEditsAndAlerts = walk("real/job-2-edits-and-alerts", [
  { press: tab("Workflow"), say: "The Job's workflow, on its canvas" },
  { press: button("Add a step after Implement"), say: "Add a step there" },
  { press: role("menuitem", "Script"), say: "A Script" },
  { type: "fmt", into: inside(NEW, role("combobox", "Script")), say: "A Command the repository declares" },
  { press: inside(NEW, button("Add", { exact: true })), say: "Add" },
  { press: card("fmt"), say: "Open the added step" },
  { look: inside(ADDED, role("group", "If it fails")), say: "Its two switches, both off" },
  { press: inside(ADDED, role("switch", "Block the Job")), say: "Block the Job on, before it fires" },
  { press: inside(ADDED, role("switch", "Self repair")), say: "Self repair on" },
  { press: inside(ADDED, button("Close")), say: "Close" },
  { press: card("fmt"), say: "Open it again" },
  { look: inside(ADDED, role("switch", "Block the Job")), say: "The switches kept what was set" },
  { press: inside(ADDED, button("Close")), say: "Close" },
  { press: button("Add a step after fmt"), say: "Another step, a Drone's" },
  { press: role("menuitem", "Drone step"), say: "A Drone step" },
  { look: inside(DRONE, role("group", "If it fails")), say: "A Drone step has Block the Job and no Self repair" },
  { type: "Format the changelog", into: inside(DRONE, role("textbox", "Brief")), say: "Its brief" },
  { press: inside(DRONE, button("Add", { exact: true })), say: "Add" },
  { press: card("Drone step"), say: "Open it" },
  { press: inside(DRONE, button("Keep for every Job")), say: "Keep it for every Job" },
  { look: inside(KEEP, role("combobox", "Runs")), say: "The Trigger editor, filled in from the step" },
  { press: inside(KEEP, button("Keep", { exact: true })), say: "Keep" },
  { press: inside(KEEP, button("Close")), say: "Close" },
  { press: button("Add a step after Drone step"), say: "A second Drone step with the same brief" },
  { press: role("menuitem", "Drone step"), say: "A Drone step" },
  { type: "Format the changelog", into: inside(DRONE, role("textbox", "Brief")), say: "The same brief" },
  { press: inside(DRONE, button("Add", { exact: true })), say: "Add" },
  { press: card("fmt"), say: "Look at the other step first" },
  { press: inside(ADDED, button("Close")), say: "Close" },
  { press: card("Drone step"), say: "Open the new one" },
  { press: inside(DRONE, button("Keep for every Job")), say: "Keep it for every Job, the name already taken" },
  { press: inside(KEEP, button("Keep", { exact: true })), say: "Kept under the next free name, never asked to Replace" },
  { press: inside(KEEP, button("Close")), say: "Close" },
  { press: tab("Overview"), say: "The Overview" },
  { look: ALERTS, say: "An alert in the lead, naming the Job and the Trigger" },
  { hover: inside(ALERTS, role("img", /Fix ready, deploy_qa/)), say: "Its state, and where it fired" },
  { press: inside(ALERTS, button(/deploy_qa/)), say: "Open the Job at its Trigger" },
  { look: role("list", "The fix"), say: "The fix held on its branch, and the files it changes" },
  { press: button("deploy/qa.sh"), say: "Open a file of the fix" },
  { look: FIX, say: "Its diff against the Job's branch, in the Job's diff sheet" },
  { press: inside(FIX, button(/\.armada\/qa\.env/)), say: "Another file of the fix, from the rail" },
  { press: inside(FIX, button("Close")), say: "Close" },
]);
