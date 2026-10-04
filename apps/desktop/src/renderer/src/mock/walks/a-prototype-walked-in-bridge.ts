// A Prototype held at Build, opened: its mock is already up for review and
// opens in Bridge's own window, so it is looked at by using it.
// `prototype-walk.test.tsx` holds the claim.

import { button, dialog, walk } from "../walk";

export const aPrototypeWalkedInBridge = walk("prototype-walked", [
  { look: dialog("Bridge's window on mock"), say: "Opening the Job opened its mock, nothing pressed" },
  { press: button("Close window"), say: "Closed, it is still the Job's to reopen" },
  { look: button("44-try-a-stacked-run-beside-the-canvas"), say: "Back on the Prototype, held at Build" },
]);
