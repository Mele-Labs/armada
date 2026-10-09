// Command Central as a cockpit. What is running is the glass, each Job and Session one instrument; a
// call that needs the owner comes forward over it and is answered from the keyboard, the ones behind
// it waiting at the edge. Over `dashboard-cockpit`.

import { button, inside, region, role, tab, walk } from "../walk";

const cockpit = region("Command Central");
const glass = role("listbox", "Running");
const waiting = role("list", "Waiting calls");
const call = (kind: RegExp) => region(kind);

const dashboardCockpit = walk("dashboard-cockpit", [
  { look: cockpit, say: "Everything running, one tile each. What is active is the glyph, and a tile that asks is lit" },
  { hover: inside(glass, role("img", "Drone working")), say: "A glyph names its state on hover" },
  { key: "j", on: cockpit, say: "j and k move along the glass, the arrows too" },
  { look: inside(glass, button("Open")), say: "The one under the cursor carries its keys: Enter opens, x stops" },
  { later: cockpit, say: "Time passes: a Drone reaches a question" },
  { look: call(/^Plan question/), say: "It comes forward over the glass, which sits back" },
  { key: "2", on: cockpit, say: "A number picks an answer" },
  { look: inside(call(/^Plan question/), role("radio", "Wrap it in place")), say: "Picked, and not sent" },
  { later: cockpit, say: "A second call arrives while the first is open" },
  { look: waiting, say: "It waits at the edge, behind the one in front" },
  { key: "Enter", on: cockpit, say: "Enter sends the answer" },
  { look: call(/^Check failed/), say: "The next call steps up" },
  { later: cockpit, say: "A Session stops to ask for a command" },
  { look: waiting, say: "Waiting again" },
  { key: "l", on: cockpit, say: "l puts this one off for later" },
  { look: call(/^Session/), say: "The Session's call comes forward in its place" },
  { key: "1", on: cockpit, say: "1 is Allow" },
  { key: "Enter", on: cockpit, say: "Enter sends it" },
  { look: inside(waiting, button(/^Check failed/)), say: "The one put off stays at the edge, dashed" },
  { key: "w", on: cockpit, say: "w brings it back" },
  { look: call(/^Check failed/), say: "Back in front" },
  { key: "Escape", on: cockpit, say: "Esc puts it off again" },
  { look: glass, say: "Nothing in front: the glass, with the call still waiting at the edge" },
  { key: "?", on: cockpit, say: "? lists every key" },
  { look: role("dialog", "Keys"), say: "The keys, by what they act on" },
  { key: "Escape", on: role("dialog", "Keys"), say: "Esc closes it" },
  { key: "]", on: cockpit, say: "] and [ switch the Dashboard's tabs" },
  { look: tab("Running"), say: "Running" },
  { key: "Alt+1", on: tab("Running"), say: "Option and a number picks a tab" },
  { look: cockpit, say: "Back on Command Central" },
]);

export { dashboardCockpit as "dashboard-cockpit" };
