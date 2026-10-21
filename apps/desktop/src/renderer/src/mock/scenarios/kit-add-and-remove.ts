// Add and Remove on Kit's Skills and Plugins pages, before Fleet runs either: a source looked at and found as a plugin,
// a skill or neither, the commands shown before anything runs, a Job started to set a source up, an install whose
// command fails, a skill to the Trash and back, and a plugin removed with what it brought. The walk `kit-add-and-remove` plays it.

import type { KitSourceFound } from "@armada/protocol";

import { manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";

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
  const served = new FakeSessionsFleet().scenario(kit);
  return {
    ...served,
    name: "kit-add-and-remove",
    says: "Kit: Add looks at a source and finds a plugin, a skill or neither, offers a Job to set it up, and Remove on skills and plugins",
    behaves: (handle) => {
      const own = served.behaves!(handle);
      return {
        ...kit.behaves!(handle),
        ...own,
        // A Session Fleet starts takes its title from its request, so the fake names it here.
        startSession: async (title, root, request) => {
          const started = await own.startSession!(title, root, request);
          if (started.ok && title !== undefined) await own.renameSession!({ session_id: started.value.id, title });
          return started.ok && title !== undefined ? { ...started, value: { ...started.value, title } } : started;
        },
      };
    },
  };
}

export const kitAddAndRemove: Scenario = build();
