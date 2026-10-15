// The Cockpit's lower deck gets a Dispatch panel between Flight control and Autopilot: one tile with N, which opens the
// centred sheet. Over `quick-dispatch`; the window reads the take from this walk's name.

import { quickDispatchDeckButton } from "../quick-dispatch-steps";

const quickDispatchDeckButtonWalk = quickDispatchDeckButton();

export { quickDispatchDeckButtonWalk as "quick-dispatch-deck-button" };
