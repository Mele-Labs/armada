// The Cockpit's lower deck gets a Dispatch panel between Flight control and Autopilot: a one-line request box on the same
// draft as the centred sheet, which grows on the first key. Over `quick-dispatch`; the window reads the take from this walk's name.

import { quickDispatchDeckBox } from "../quick-dispatch-steps";

const quickDispatchDeckBoxWalk = quickDispatchDeckBox();

export { quickDispatchDeckBoxWalk as "quick-dispatch-deck-box" };
