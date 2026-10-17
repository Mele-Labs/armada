// The inspect card, the one card every rich hover opens: a map drone, the plan limits, a merge queue chip and a Session's context
// tank, each with the same head, name, state band, facts and keys. Over `cockpit-ship-flightdeck`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs and Sessions");
const queue = region("Merge queue");
const context = button(/^Context 61%/);

const inspectCard = walk("cockpit-ship-flightdeck", [
  { hover: role("option", /^Cache the manifest read between dispatches, Job/), say: "A drone on the map opens a card: kind, name, state, what it is doing, and the keys that work on it" },
  { hover: button(/^5-hour limit 34%/), say: "The plan limits open the same card: the same head, the same surface, the same keys" },
  { press: inside(list, button("Migration notes")), say: "A Session opens in the area, and the merge queue shows its repository" },
  { hover: inside(queue, button(/#1890/)), say: "A merge queue chip: pull request, state, checks, place and branch" },
  { hover: context, say: "The Session's context tank: window, state and what fills it" },
  { press: context, say: "A press pins it. Focus moves into the card, and Esc or a press outside closes it" },
  { key: "Escape", on: role("dialog", "Context"), say: "Esc closes it and focus goes back to the tank" },
]);

export { inspectCard as "inspect-card" };
