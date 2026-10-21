// A link dropped anywhere in Bridge, and Add and Remove on Kit's pages, before Fleet runs any of it: a link looked at
// and found as a plugin, a skill or neither, with the commands shown before anything runs; installed in Kit, sent to the
// Job open, or a Job started with it; an install whose command fails, a skill to the Trash and back, and a plugin
// removed with what it brought. The walk `kit-add-and-remove` plays it.

import type { KitSourceFound } from "@armada/protocol";

import { manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { MockDraft, Scenario } from "../moment";
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

/**
 * The Sessions walks' store, with a Job started from a request titled by that request's first sentence, as Fleet's
 * proposer would name it a moment later. The store's own start leaves it untitled.
 */
const titledSessions: NonNullable<MockDraft["sessions"]> = (board, more) => {
  const store = s200Sessions.draft!.sessions!(board, more);
  const start = store.start.bind(store);
  store.start = async (tag, root, request, tuning, after) => {
    const id = await start(tag, root, request, tuning, after);
    const title = request?.split(/\.\s/)[0];
    if (id !== undefined && title !== undefined) store.rename?.(id, title);
    return id;
  };
  return store;
};

function build(): Scenario {
  const kit = manifesting({
    kitInventory: PLUGIN_KIT_INVENTORY,
    kitPieces: {
      found: FOUND,
      failing: {
        // The marketplace went on, and the plugin's install failed after it, so Fleet took the marketplace back off.
        "example/broken-plugin": {
          error: `Installing plugin "broken-plugin@broken-plugin"...\nError: Failed to install plugin: .claude-plugin/plugin.json is missing the required field "name"`,
          at: 1,
          takenBack: ["the broken-plugin marketplace"],
        },
      },
    },
  });
  // The Cockpit's Jobs, drawn ahead of Fleet as the Sessions walks draw them, so a Job can be open when a link lands.
  return {
    ...kit,
    ...(s200Sessions.held === undefined ? {} : { held: s200Sessions.held }),
    draft: { ...s200Sessions.draft, sessions: titledSessions },
    name: "kit-add-and-remove",
    says: "A link dropped anywhere opens a panel that looks at it: install it in Kit, start a Job with it, or add it to the Job open",
  };
}

export const kitAddAndRemove: Scenario = build();
