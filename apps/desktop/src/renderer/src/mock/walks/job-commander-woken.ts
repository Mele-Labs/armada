// Fleet wakes a Job's Commander and nobody is asked: T7's Drone ends at the turn cap with nothing
// handed in, and the Commander reads why, splits the task, raises the cap and restarts it. Each act
// is a row of its thread and a mark on what it changed, which is its hand and no Drone's or person's.
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
  { look: inside(commander, text("Turn cap 300 → 600")), say: "Each act is a row: the cap raised" },
  { look: inside(commander, text("T7 split into T7, T11")), say: "The task split" },
  { hover: inside(commander, role("img", "Split task")), say: "A mark names the act" },
  { look: inside(commander, text("T7 restarted")), say: "And T7 restarted on a Drone of its own" },
  { look: inside(now, role("img", "Commander restarted T7")), say: "The Now panel marks the Drone it started" },
  { look: role("img", "Commander split it off T7"), say: "On the canvas, T11 wears the Commander's mark" },
  { press: tab("Plan"), say: "The Plan tab" },
  { look: role("img", "Commander split it off T7"), say: "T11 is marked there too" },
  { look: role("img", "Commander split T11 off it"), say: "And the task it was split from" },
]);

export { w as "job-commander-woken" };
