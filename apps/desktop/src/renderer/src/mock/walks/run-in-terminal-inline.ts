// A command a Session gave the person opens a terminal that takes keys, under the block he ran. Over the
// `session-run-in-terminal` scenario. Take one of two: the other docks the pane above the message box.

import { walk } from "../walk";
import { PLACE_KEY } from "../terminal-run/place";
import { deployWalk } from "../terminal-run/steps";

const inline = { ...walk("session-run-in-terminal", deployWalk("inline")), remembering: { [PLACE_KEY]: "inline" } };

export { inline as "run-in-terminal-inline" };
