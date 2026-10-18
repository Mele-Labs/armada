// Every slot taken when a Session's first write wants one: its call offers waiting or adding a slot, and Add a slot puts the Session in
// a new bay. The Bays panel adds a bay of its own. Over `slot-full-ask`.

import { button, inside, region, role, walk } from "../walk";

const bays = region("Bays");
const needs = region("Needs you");
const choice = (name: string) => inside(needs, role("radio", name, { exact: true }));

const slotFullAsk = walk("slot-full-ask", [
  { look: inside(inside(region("Work", { exact: true }), button(/Chart the timeline/)), role("img", "Waiting on you")), say: "Its row reads Waiting on you, though its turn is running" },
  { press: inside(region("Work", { exact: true }), button(/Chart the timeline/)), say: "Every slot is in use: its row opens the Session" },
  { press: inside(needs, button(/^Needs you/)), say: "The same call is in its Needs you panel" },
  { press: inside(needs, button("Open Every slot is in use")), say: "A press on its row opens it" },
  { look: choice("Wait for a slot"), say: "1 waits for a slot" },
  { key: "2", on: choice("Add a slot"), say: "2 picks Add a slot" },
  { press: inside(needs, button("Answer")), say: "Sent" },
  { press: inside(region(/^Session: Chart the timeline/), button("Back", { exact: true })), say: "Back to the Bays" },
  { look: inside(bays, button(/^slot-7, armada, Held, Chart the timeline/)), say: "A new bay, held by the Session" },
  { hover: inside(bays, button("Add a slot to armada")), say: "Add a slot" },
  { press: inside(bays, button("Add a slot to armada")), say: "A bay from the panel" },
  { look: inside(bays, button(/^slot-8, armada, Not made yet/)), say: "Free, and made at its first lease" },
]);

export { slotFullAsk as "slot-full-ask" };
