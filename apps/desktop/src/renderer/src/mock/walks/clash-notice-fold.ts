// A clash that names many files: one bar folded to a line with the other holder as a chip, opened to the files, the first three and a
// chip for the rest, every file on a press, a chip that folds them back, and the bar's own press that folds it again. Over the
// `clash-notice-fold` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const notice = region("Write clash");
const bar = button("Write clash");
const files = inside(notice, role("list", /^Conflicting files with/));

const folded = walk("clash-notice-fold", [
  toSessions,
  { press: inside(region("Work", { exact: true }), button(/Checkout card sync/)), say: "The Session, in a clash with main" },
  { look: inside(notice, bar), say: "One bar, folded to a line with who it clashes with" },
  { press: inside(notice, bar), say: "Open, the files that conflict" },
  { look: files, say: "Three files, and a chip for the rest" },
  { press: inside(notice, button("34 more")), say: "Every file" },
  { look: files, say: "The chip now folds them back" },
  { press: inside(notice, button("Fewer")), say: "Three again" },
  { press: inside(notice, bar), say: "The bar folds to one line" },
  { look: inside(notice, bar), say: "Only the line is left" },
  { press: inside(notice, bar), say: "Open again, the files as they were" },
]);

export { folded as "clash-notice-fold" };
