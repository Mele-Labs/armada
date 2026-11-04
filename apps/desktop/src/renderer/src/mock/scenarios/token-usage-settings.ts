// Settings Fleet does not serve yet, drawn as Fleet would serve them: Kit › Skills with a switch on each skill for
// whether Jobs get it (two already off, and a repository's own skill and Armada's with none), Fleet › Token use with
// three Machine settings, and Fleet › Repository with the connectors switch saved off. The walk `tokenUsageSettings`
// plays it.

import type { KitInventory, SetupItem } from "@armada/protocol";
import { MOCK_SETTINGS, TOKEN_USE_SETTINGS } from "@armada/settings/fake";

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

const HOME = "/Users/user/.agent";

/** Skills beside the plugin inventory's: two the settings switch off, and one the repository brings. */
const MORE_SKILLS: SetupItem[] = [
  {
    name: "gitnexus-cli",
    says: "Run GitNexus commands to index a repository and read its graph.",
    source: `${HOME}/skills/gitnexus-cli`,
  },
  {
    name: "pptx",
    says: "Read, write and edit slide decks.",
    source: `${HOME}/plugins/cache/anthropic-skills/1.0.0/skills/pptx`,
    plugin: "anthropic-skills",
  },
  {
    name: "release-notes",
    says: "Write the release notes for a tagged version.",
    source: "/Users/user/develop/armada/.agent/skills/release-notes",
  },
];

const INVENTORY: KitInventory = {
  ...PLUGIN_KIT_INVENTORY,
  kinds: PLUGIN_KIT_INVENTORY.kinds.map((one) =>
    one.kind === "skills" && one.read.what === "read"
      ? {
          ...one,
          read: {
            ...one.read,
            items: [
              ...one.read.items,
              ...MORE_SKILLS,
            ],
          },
        }
      : one,
  ),
};

function build(): Scenario {
  const base = manifesting({
    kitInventory: INVENTORY,
    kitServers: KIT_SERVERS,
  });
  return {
    ...base,
    name: "token-usage-settings",
    says: "Kit › Skills with a switch for Jobs on each skill, Fleet › Token use and Fleet › Repository, served before Fleet has them",
    state: {
      ...base.state,
      settings: {
        ...MOCK_SETTINGS,
        settings: [
          ...MOCK_SETTINGS.settings,
          ...TOKEN_USE_SETTINGS,
        ],
      },
    },
  };
}

export const tokenUsageSettings: Scenario = build();
