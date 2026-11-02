// A Job's first write finds every slot held: its call lists who holds each, the quietest first, and Free gives one to the Job. Over
// `free-held-slot`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const bays = region("Bays");
const needs = region("Needs you");
const held = inside(needs, region("Slots"));

const freeHeldSlot = walk("free-held-slot", [
  { press: deckRow(/Chart the timeline/), say: "Every slot is in use: its row opens the Job" },
  { press: inside(needs, button(/^Needs you/)), say: "The call is in its Needs you panel" },
  { look: inside(held, role("listitem", /^Slot 1, Move the run ledger/)), say: "Under Wait, one row for each slot held: the quietest first" },
  { hover: inside(held, button("Free slot 2")), say: "Free is off where the holder has uncommitted work" },
  { press: inside(held, button("Create a slot")), say: "The last row creates a slot for this Job. That can take a while" },
  { later: inside(held, role("listitem", "Creating a slot")), say: "The row counts up and Free stays on" },
  { look: inside(held, text(/^Slot not created/)), say: "If it fails, Fleet's words go under the row and Create is back" },
  { press: inside(held, button("Free slot 1")), say: "Free gives the quietest slot to this Job" },
  { press: inside(region(/^Job: Chart the timeline/), button("Back", { exact: true })), say: "Back to the Bays" },
  { look: inside(bays, button(/^slot-1, armada, Held, Chart the timeline/)), say: "The Job holds slot 1 and has gone on" },
  { look: deckRow(/Move the run ledger/), say: "Its old holder stays in the list, and leases a new slot when it next writes" },
  { press: deckRow(/Wire the retro sheet/), say: "The other waiting Job" },
  { press: inside(needs, button(/^Needs you/)), say: "Slot 1 has gone from its call" },
  { press: inside(held, button("Create a slot")), say: "Create, on this one" },
  { press: deckRow(/Chart the timeline/), say: "Leave while it runs" },
  { look: deckRow(/Wire the retro sheet/), say: "Its row stops asking for you: you have answered" },
  { press: deckRow(/Wire the retro sheet/), say: "Come back" },
  { later: inside(held, role("listitem", "Creating a slot")), say: "The wait is still here, still counting" },
  { press: inside(region(/^Job: Wire the retro sheet/), button("Back", { exact: true })), say: "Back to the Bays" },
  { look: inside(bays, button(/^slot-4, armada, Held, Wire the retro sheet/)), say: "Once made, the Job holds the new slot and goes on, with no press from you" },
]);

export { freeHeldSlot as "free-held-slot" };
