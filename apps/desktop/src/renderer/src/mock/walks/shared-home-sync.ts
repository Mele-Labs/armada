// Sync in Kit: it adds every skill only on this machine to the home, and a skill it cannot add stays,
// told in a toast that names it. Over `shared-home`.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const MARK = (said: string) => inside(region("Skills"), role("img", said, { exact: true }));
const FILTER = inside(region("Skills"), role("searchbox", "Filter Skills"));

const sharedHomeSync = walk("shared-home", [
  ...toSystemPage("Kit", "Kit"),
  { type: "release-notes", into: FILTER, say: "A skill only on this machine" },
  { look: MARK("Only on this machine"), say: "release-notes is only on this machine" },
  { type: "deploy", into: FILTER, say: "Another skill only on this machine" },
  { look: MARK("Only on this machine"), say: "deploy is only on this machine" },
  { press: button("Sync"), say: "Sync" },
  { look: text("deploy stays on this machine"), say: "deploy stays on this machine, and the toast says why" },
  { look: MARK("Only on this machine"), say: "deploy is still only on this machine" },
  { type: "release-notes", into: FILTER, say: "Back to release-notes" },
  { look: MARK("Same as the home"), say: "release-notes is in the home" },
]);

export { sharedHomeSync as "shared-home-sync" };
