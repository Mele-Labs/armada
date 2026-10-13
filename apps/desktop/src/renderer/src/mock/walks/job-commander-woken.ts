// Fleet wakes a Job's Commander and nobody is asked: T7's Drone ends at the turn cap with nothing
// handed in, and the Commander reads why, splits the task, raises the cap and restarts it. Each act
// is a row of its thread and a mark on what it changed, which is its hand and no Drone's or person's.
// The rows and marks say which act and whose hand, as the wire does, and carry no figure.
// Over `job-commander-woken`: each `later` is Fleet's next moment.

import { inside, region, role, tab, text, walk } from "../walk";

const commander = role("region", "Commander", { exact: true });
const now = region("Now");

const w = walk("job-commander-woken", [
  { look: commander, say: "The Job's Commander, across the top where the lead was" },
  { look: inside(now, text(/Edit crates\/ipc\/src\/asks.rs/)), say: "T7's Drone at work" },
  { later: commander, say: "T7's Drone ends at the turn cap with nothing handed in" },
  { look: inside(commander, text("Drone on T7 ended with nothing handed in")), say: "Fleet wakes it, in a row of its own that names what woke it" },
  { later: commander, say: "It reads why, and acts inside its reach" },
  { look: inside(commander, text("Turn cap")), say: "Each act is a row: the cap raised" },
  { look: inside(commander, text("add_task")), say: "The split is the two calls it is: the task narrowed, a new one added" },
  { hover: inside(commander, role("img", "Raised cap")), say: "A mark names the act" },
  { look: inside(commander, text("Task restarted")), say: "And T7 restarted on a Drone of its own" },
  { look: inside(now, role("img", "Commander")), say: "The Now panel marks the Drone on a task it touched" },
  { look: role("img", "Commander"), say: "On the canvas, the tasks it touched wear the Commander's mark" },
  { press: tab("Plan"), say: "The Plan tab" },
  { look: role("img", "Commander"), say: "The same tasks are marked there" },
]);

export { w as "job-commander-woken" };
