// A Job waiting on the owner is paused: its row dims and stops pulsing, its call goes, Fleet's notice waits in its thread, and a note typed to it asks
// to Resume and send first. Over `pausing-a-session`.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const row = inside(list, button("Rebuild the stage grid"));
const stage = region(/^Job: Rebuild the stage grid/);

const pausingQuietsACall = walk("pausing-a-session", [
  { look: row, say: "This Job waits on you: its row is yellow and pulses" },
  { key: "ContextMenu", on: row, say: "You open its menu" },
  { press: role("menuitem", "Pause"), say: "You pause it" },
  { look: inside(list, role("img", "Paused")), say: "The row dims and stops pulsing. Its call is gone from Needs you, the dock count, the alerts and the map" },
  { press: row, say: "You open it" },
  { look: inside(stage, text(/main moved 3 commits/)), say: "Fleet's notice waits in the thread. The Job stays paused" },
  { type: "Use sixteen columns\n", into: inside(stage, role("textbox", "Message")), say: "You write a note to the paused Job" },
  { look: dialog("Job is paused"), say: "Resume and send, or Cancel. Enter confirms, Escape cancels, and Cancel keeps your note in the box" },
  { press: button("Resume and send"), say: "You press Resume and send" },
  { look: row, say: "The pause mark is gone and the Job works on the note" },
]);

export { pausingQuietsACall as "pausing-quiets-a-call" };
