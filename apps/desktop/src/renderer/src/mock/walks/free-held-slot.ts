// A Job's first write finds every slot held: its call lists who holds each, the quietest first, and Free gives one to the Job. Over
// `free-held-slot`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const bays = region("Bays");
const needs = region("Needs you");
const held = inside(needs, region("Held slots"));

const freeHeldSlot = walk("free-held-slot", [
  { press: deckRow(/Chart the timeline/), say: "Every slot is in use: its row opens the Job" },
  { press: inside(needs, button(/^Needs you/)), say: "The call is in its Needs you panel" },
  { look: inside(held, role("listitem", /^Slot 1, Migration notes/)), say: "Under the two choices, one row for each slot held: the quietest first" },
  { hover: inside(held, button("Free slot 2")), say: "Free is off where the holder has uncommitted work" },
  { press: inside(held, button("Free slot 1")), say: "Free gives the quietest slot to this Job" },
  { press: inside(region(/^Job: Chart the timeline/), button("Back", { exact: true })), say: "Back to the Bays" },
  { look: inside(bays, button(/^slot-1, armada, Held, Chart the timeline/)), say: "The Job holds slot 1 and has gone on" },
  { look: deckRow(/Migration notes/), say: "Its old holder stays in the list, and leases a new slot when it next writes" },
]);

export { freeHeldSlot as "free-held-slot" };
