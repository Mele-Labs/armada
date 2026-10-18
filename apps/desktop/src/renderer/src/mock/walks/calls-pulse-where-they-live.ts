// A call no Job's row holds pulses the place that already shows it, in the tint and pulse a calling row has. Main red and a
// failed pull request each pulse their light and chip in the merge queue; no row in Work does. c goes to the first of them
// when no row needs him. Over `calls-pulse-where-they-live`.

import { button, deckRow, inside, region, walk } from "../walk";

const queue = region("Merge queue");
const list = region("Jobs", { exact: true });

const callsPulseWhereTheyLive = walk("calls-pulse-where-they-live", [
  { look: inside(queue, button(/^Main is red/)), say: "Main is red and nobody is on it. Its light in the merge queue is tinted and pulses" },
  { look: inside(queue, button(/^#1896/)), say: "#1896 failed its checks, with nobody on it. Its chip pulses the same way" },
  { look: inside(queue, button(/^#1892/)), say: "#1892 passed and asks nothing: its chip stays quiet" },
  { look: deckRow("Pin the store clock"), say: "The one Job here only told him something. No row pulses" },
  { key: "c", on: list, say: "c goes to the first row that needs him, and with none to the first place that holds a call: main's light" },
]);

export { callsPulseWhereTheyLive as "calls-pulse-where-they-live" };
