// Pause from a Session row's menu and from Flight control: the row's menu opens on the ContextMenu key (and on a right press), Pause marks the
// row paused, the menu reads Resume, and the key P does the same from Flight control. Over `pausing-a-session`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const row = inside(list, button("Pin the store clock"));
const paused = inside(list, role("img", "Paused"));

const pausingASession = walk("pausing-a-session", [
  { press: row, say: "A Job working, open on the stage" },
  { key: "ContextMenu", on: row, say: "The row's menu opens on the ContextMenu key" },
  { look: role("menuitem", "Auto Pilot"), say: "Pause, Auto Pilot, then Close" },
  { press: role("menuitem", "Pause"), say: "Pause" },
  { look: paused, say: "The row stays in the list with the paused mark" },
  { key: "ContextMenu", on: row, say: "The menu again" },
  { press: role("menuitem", "Resume"), say: "Resume takes a slot again" },
  { key: "p", on: controls, say: "P pauses it from Flight control too" },
  { look: paused, say: "Paused again" },
]);

export { pausingASession as "pausing-a-session" };
