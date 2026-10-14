// Kit on a second machine against the shared home: the account and Sync in the band, a mark on each
// row for how it stands, the acts a row that is on one side only offers, and the two versions of a
// row that differs. Over `shared-home`.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const MARK = (kind: string, said: string) => inside(region(kind), role("img", said, { exact: true }));

const sharedHome = walk("shared-home", [
  { press: button("Kit", { exact: true }), say: "Kit" },
  { look: text("nickmele"), say: "The GitHub account the home belongs to, and Sync beside it" },
  { hover: text("nickmele"), say: "The home" },
  { look: MARK("Workflows", "Same as the home"), say: "The workflows match: a mark that recedes" },
  { look: MARK("Skills", "Same as the home"), say: "So do the skills" },
  { hover: MARK("MCP servers", "Needs its key on this machine"), say: "A server with no key on this machine" },
  { look: MARK("Allowlist", "Only on this machine"), say: "A command only this machine allows" },
  { look: MARK("Allowlist", "Only in the home"), say: "A command only the home has" },
  { press: button("Add to the home git log --oneline"), say: "Add to the home" },
  { press: button("Add to this machine gh run view"), say: "Add to this machine" },
  { look: MARK("Allowlist", "Same as the home"), say: "Both settle to the same" },
  { look: MARK("Memory", "Only in the home"), say: "Memory entries only the home has, matched by repository name" },
  { press: button("Add to this machine Never generate load"), say: "Add one to this machine" },
  { hover: button("Differs from the home, AGENTS.md"), say: "The global instructions differ" },
  { press: button("Differs from the home, AGENTS.md"), say: "Open both versions" },
  { look: inside(dialog("AGENTS.md"), region("This machine")), say: "This machine's version" },
  { look: inside(dialog("AGENTS.md"), region("The home")), say: "The home's, with the lines that differ lit on each side" },
  { press: inside(dialog("AGENTS.md"), button(/^Use the home.s$/)), say: "Pick a side" },
  { look: MARK("Global instructions", "Same as the home"), say: "The row is the same" },
  { press: button("Sync"), say: "Sync" },
]);

export { sharedHome as "shared-home" };
