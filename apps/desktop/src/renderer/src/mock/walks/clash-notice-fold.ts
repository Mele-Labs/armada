// A clash that names many files: the first three, a chip for the rest, every file on a press, a chip that folds
// them back, and a header that folds the whole panel. Over the `clash-notice-fold` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const notice = region("Write clash");
const files = inside(notice, role("list", "Conflicting files"));

const folded = walk("clash-notice-fold", [
  toSessions,
  { press: inside(region("Sessions"), button(/Checkout card sync/)), say: "The Session, in a clash with main" },
  { look: files, say: "Three files, and a chip for the rest" },
  { press: inside(notice, button("34 more")), say: "Every file" },
  { look: files, say: "The chip now folds them back" },
  { press: inside(notice, button("Fewer")), say: "Three again" },
  { press: inside(notice, button("Conflicts with")), say: "The header folds the whole panel to one line" },
  { look: notice, say: "Only the header is left" },
  { press: inside(notice, button("Conflicts with")), say: "Open again, the files as they were" },
]);

export { folded as "clash-notice-fold" };
