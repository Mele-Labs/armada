// The inspect card, the one card every rich hover opens: a map drone, a list row, a bay, a call, the plan limits, a merge queue chip and a Session's context
// tank, each with the same head, name, state band, facts and keys. Over `cockpit-ship-flightdeck`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Work", { exact: true });
const queue = region("Merge queue");
const context = button(/^Context 61%/);

const inspectCard = walk("cockpit-ship-flightdeck", [
  { hover: role("option", /^Pin the store clock, Session/), say: "A drone on the map opens a card: kind, name, state, what it is doing, and the keys that work on it" },
  { hover: inside(list, button("Changelog script")), say: "A row of Work opens the same card: kind, repository, state, bay and last word, with Enter to open" },
  { hover: inside(region("Bays"), button(/^slot-3, armada, Held/)), say: "A bay: its holder, branch and how long it has held it" },
  { hover: inside(list, button("Cache key")), say: "A row that is calling him names the call in the same card: what is calling in its own words, where, and how long ago" },
  { hover: button(/^5-hour limit 34%/), say: "The plan limits open the same card: the same head, the same surface, the same keys" },
  { press: inside(list, button("Migration notes")), say: "A Session opens in the area, and the merge queue shows its repository" },
  { hover: inside(queue, button(/#1890/)), say: "A merge queue chip: pull request, state, checks, place and branch" },
  { hover: context, say: "The Session's context tank: window, state and what fills it" },
  { press: context, say: "A press pins it. Focus moves into the card, and Esc or a press outside closes it" },
  { key: "Escape", on: role("dialog", "Context"), say: "Esc closes it and focus goes back to the tank" },
]);

export { inspectCard as "inspect-card" };
