// Kit on a second machine against the shared home: the account and Sync in the band, a mark on each
// row for how it stands, the acts a row that is on one side only offers, and the two versions of a
// row that differs. Over `shared-home`.

import { button, dialog, inside, region, role, text, toSystemPage, walk } from "../walk";

const MARK = (kind: string, said: string) => inside(region(kind), role("img", said, { exact: true }));

const sharedHome = walk("shared-home", [
  ...toSystemPage("Kit", "Kit"),
  { look: region("Skills"), say: "A long list scrolls inside its tile" },
  { look: text("nickmele"), say: "The GitHub account the home belongs to, and Sync beside it" },
  { hover: text("nickmele"), say: "The home" },
  { look: MARK("Skills", "Same as the home"), say: "The skills match: a mark that recedes" },
  ...toSystemPage("Servers", "Servers"),
  { hover: MARK("Servers", "Needs its key on this machine"), say: "A server with no key on this machine" },
  ...toSystemPage("Allowlist", "Allowlist"),
  { look: MARK("Allowlist", "Only on this machine"), say: "A command only this machine allows" },
  { look: MARK("Allowlist", "Only in the home"), say: "A command only the home has" },
  { press: button("Add to the home git log --oneline"), say: "Add to the home" },
  { press: button("Add to this machine gh run view"), say: "Add to this machine" },
  { look: MARK("Allowlist", "Same as the home"), say: "Both settle to the same" },
  ...toSystemPage("Instructions", "Instructions"),
  { hover: button("Differs from the home, AGENTS.md"), say: "The global instructions differ" },
  { press: button("Differs from the home, AGENTS.md"), say: "Open both versions" },
  { look: inside(dialog("AGENTS.md"), region("This machine")), say: "This machine's version" },
  { look: inside(dialog("AGENTS.md"), region("The home")), say: "The home's, with the lines that differ lit on each side" },
  { press: inside(dialog("AGENTS.md"), button(/^Use the home.s$/)), say: "Pick a side" },
  { look: MARK("Instructions", "Same as the home"), say: "The row is the same" },
  { press: button("Sync"), say: "Sync" },
]);

export { sharedHome as "shared-home" };
