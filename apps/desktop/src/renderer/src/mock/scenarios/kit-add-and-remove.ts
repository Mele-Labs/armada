// A link dropped anywhere in Bridge, and Add and Remove on Kit's pages, before Fleet runs any of it: a link looked at
// and found as a plugin, a skill or neither, with the commands shown before anything runs; installed in Kit, sent to the
// Job open, or a Job started with it; an install whose command fails, a skill to the Trash and back, and a plugin
// removed with what it brought. The walk `kit-add-and-remove` plays it.

import type { KitSourceFound } from "@armada/protocol";

import { manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const plugin = (source: string, repository: string, name: string, skills: string[]): [string, KitSourceFound] => [
  source,
  { found: "plugin", name, marketplace: name, repository, skills, commands: [`claude plugin marketplace add ${repository}`, `claude plugin install ${name}@${name}`] },
];

/** What a look finds, by the source as pasted. Anything else is found unknown. */
const FOUND: Record<string, KitSourceFound> = Object.fromEntries([
  plugin("https://impeccable.style/", "pbakaus/impeccable", "impeccable", ["impeccable"]),
  plugin("example/broken-plugin", "example/broken-plugin", "broken-plugin", []),
  ["example/release-notes", { found: "skill", name: "release-notes", repository: "example/release-notes", commands: ["npx skills add example/release-notes"] }],
]);

function build(): Scenario {
  const kit = manifesting({
    kitInventory: PLUGIN_KIT_INVENTORY,
    kitPieces: {
      found: FOUND,
      failing: {
        "example/broken-plugin": `Cloning into '/Users/user/.agent/plugins/marketplaces/broken-plugin'...\nError: Failed to add marketplace: .claude-plugin/marketplace.json is not valid JSON at line 4`,
      },
    },
  });
  // The Cockpit's Jobs, drawn ahead of Fleet as the Sessions walks draw them, so a Job can be open when a link lands.
  return {
    ...kit,
    ...(s200Sessions.held === undefined ? {} : { held: s200Sessions.held }),
    ...(s200Sessions.draft === undefined ? {} : { draft: s200Sessions.draft }),
    name: "kit-add-and-remove",
    says: "A link dropped anywhere opens a panel that looks at it: install it in Kit, start a Job with it, or add it to the Job open",
  };
}

export const kitAddAndRemove: Scenario = build();
