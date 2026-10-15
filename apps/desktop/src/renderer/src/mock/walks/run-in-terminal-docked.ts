// A command a Session gave the person opens a terminal that takes keys, docked above the Session's message
// box with a handle on its top edge. Over the `session-run-in-terminal` scenario. Take two of two: the other
// opens the pane under the block he ran.

import { walk } from "../walk";
import { PLACE_KEY } from "../terminal-run/place";
import { deployWalk } from "../terminal-run/steps";

const docked = { ...walk("session-run-in-terminal", deployWalk("docked")), remembering: { [PLACE_KEY]: "docked" } };

export { docked as "run-in-terminal-docked" };
