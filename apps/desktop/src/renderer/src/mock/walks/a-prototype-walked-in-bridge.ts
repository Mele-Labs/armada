// A Prototype held at Build, opened: its mock is already up for review and
// opens in Bridge's own window, so it is looked at by using it.
// `prototype-walk.test.tsx` holds the claim.

import { button, dialog, walk } from "../walk";

export const aPrototypeWalkedInBridge = walk("prototype-walked", [
  { look: dialog("Bridge's window on mock"), say: "Opening the Job opened its mock, nothing pressed" },
  { press: button("Close window"), say: "Close it whenever you like" },
  { press: button("Walk in Bridge"), say: "The Job's lead opens it again, as often as you want" },
  { look: dialog("Bridge's window on mock"), say: "Back, and the app beside it still works" },
]);
