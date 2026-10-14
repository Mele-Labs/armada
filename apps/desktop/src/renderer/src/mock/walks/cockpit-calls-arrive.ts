// A call no longer covers the work. The Cockpit on Active shows what is live and nothing in front of it;
// a call that needs the owner arrives in a tray along the foot of the panel, rings in its hue, and
// waits there until he opens it with `w` or a press. Answering it clears the tray, and the next call
// arrives the same way. Over `cockpit-calls-arrive`.

import { button, inside, region, role, tab, walk } from "../walk";

const cockpit = region("Dashboard");
const glass = role("listbox", "Tiles");
const plan = region(/^Plan question/);
const check = region(/^Check failed/);

const cockpitCallsArrive = walk("cockpit-calls-arrive", [
  { look: tab("Active"), say: "One filter for what is live, and the one the Cockpit opens on" },
  { press: button("Grid"), say: "Grid, the tiles view" },
  { look: inside(glass, role("option", /Cache the manifest read/)), say: "Two Jobs at work, and nothing in front of them" },
  { later: cockpit, say: "Time passes, and nothing is new" },
  { later: cockpit, say: "A Drone reaches a question" },
  { look: button(/^Plan question/), say: "It arrives in the tray along the foot: it slides up and its hue rings out twice, then it waits. The work is uncovered" },
  { look: inside(glass, role("option", /Cache the manifest read/)), say: "The tiles stay pressable and the keys stay the glass's" },
  { key: "w", on: cockpit, say: "w opens it, as pressing it does" },
  { look: plan, say: "The whole card, centred over the panel, with every key it had" },
  { key: "2", on: cockpit, say: "A number picks an answer" },
  { key: "Enter", on: cockpit, say: "Enter sends it" },
  { look: glass, say: "The card goes and the tray is empty" },
  { later: cockpit, say: "A Check fails on another Job" },
  { look: button(/^Check failed/), say: "The next call arrives the same way" },
  { press: button(/^Check failed/), say: "A press opens it" },
  { look: check, say: "The failing tests and its output, on the card" },
  { key: "l", on: cockpit, say: "l puts it off" },
  { look: button(/^Check failed/), say: "A call put off waits in the tray, dashed, and does not ring again" },
]);

export { cockpitCallsArrive };
