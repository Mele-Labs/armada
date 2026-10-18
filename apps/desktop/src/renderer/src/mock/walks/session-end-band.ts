// The agent's last word is a band on the composer, in the done hue, drawn as the Needs you band is. A press rides it up as the header of a panel
// holding what was made and the two ways on, and the thread dims; Esc folds it. It never opens by itself. Play it under `&take=band`: under the
// default take the callout stays in the thread's footer.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const band = inside(region("Nothing left to do"), button("Nothing left to do"));

const sessionEndBand = walk("session-all-done", [
  toSessions,
  { press: inside(region("Work", { exact: true }), button(/Pin the store clock/)), say: "A Session whose agent has nothing left to do" },
  { look: band, say: "A band on the composer, folded" },
  { press: band, say: "It rides up as the panel's header, and the thread dims" },
  { look: inside(region("Nothing left to do"), button("End session")), say: "What was made, and the two ways on" },
  { key: "Escape", on: inside(region("Nothing left to do"), button("End session")), say: "Esc folds it" },
  { look: band, say: "The band is back on the composer" },
]);

export { sessionEndBand as "session-end-band" };
