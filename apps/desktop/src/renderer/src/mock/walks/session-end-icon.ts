// End job in the More menu of a Session in the done state: the act wears `power-off` and the Done mark stays `power`, so
// the two no longer stand identical. At 1512 by 949 with the Job open, where the breakdown opens
// leftward inside the panel. The done card's own End job is put away first, so the tab's is the one pointed at.
// Over the `session-head` scenario.

import { button, inside, region, role, tab, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const head = region("Job s-c9e3a5f1");
// The Job's head is its Thread tab: the state mark, the name and More.
const job = tab(/^Thread: /);
const sessions = region("Jobs", {
  exact: true,
});

const sessionEndIcon = walk(
  "session-head",
  [
    toSessions,
    {
      press: inside(sessions, role("button", /Release notes draft/)),
      say: "A Job that said it is done",
    },
    {
      look: region("Nothing left to do"),
      say: "Its card is at the end of the thread",
    },
    {
      press: inside(region("Nothing left to do"), button("Keep going")),
      say: "Keep going puts the card away, and the state stays Done",
    },
    {
      look: inside(job, role("img", /^Done/)),
      say: "The Done mark stands first",
    },
    {
      hover: inside(head, role("button", "Context")),
      say: "The breakdown opens leftward, inside the panel",
    },
    {
      press: inside(region(/^Pane: Thread/), button("More")),
      say: "More on the Job's tab",
    },
    {
      hover: role("menuitem", /^End job/),
      say: "End job",
    },
  ],
  {
    width: 1512,
    height: 949,
  },
);

export { sessionEndIcon as "session-end-icon" };
