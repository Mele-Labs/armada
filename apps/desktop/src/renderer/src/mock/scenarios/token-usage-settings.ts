// Settings Fleet does not serve yet, drawn as Fleet would serve them, over three repositories: a skill's detail in
// Kit › Skills listing the repositories whose Jobs get it (none for a repository's own skill or Armada's), Fleet ›
// Token use with three Machine settings, and Fleet › Repository with a connectors switch per repository. The walk
// `tokenUsageSettings` plays it.

import type { KitInventory, RepositorySummary, SetupItem } from "@armada/protocol";
import { MANIFEST_ID, repository } from "@armada/screens/src/fixtures/repository";
import { MOCK_SETTINGS, tokenUseSettings } from "@armada/settings/fake";

import { KIT_SERVERS, manifesting, PLUGIN_KIT_INVENTORY } from "../manifest-fake";
import type { Scenario } from "../moment";

const HOME = "/Users/user/.agent";

/** Skills beside the plugin inventory's: one of the person's, one a plugin brings, and one the repository brings. */
const MORE_SKILLS: SetupItem[] = [
  {
    name: "gitnexus-cli",
    says: "Run GitNexus commands to index a repository and read its graph.",
    source: `${HOME}/skills/gitnexus-cli`,
  },
  {
    name: "pptx",
    says: "Read, write and edit slide decks.",
    source: `${HOME}/plugins/cache/office-skills/1.0.0/skills/pptx`,
    plugin: "office-skills",
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

/** Two more repositories beside armada, so each switch set per repository has more than one row. */
const ANOTHER = (id: string, name: string): RepositorySummary => {
  const one = repository();
  return {
    root: `/Users/user/${name}`,
    records_root: `/Users/user/Library/Application Support/Armada/records/${name}`,
    manifest: {
      ...one.manifest!,
      id,
      repository: name,
      path: `${name}/armada.yml`,
      records_root: `/Users/user/Library/Application Support/Armada/records/${name}`,
    },
  };
};
const STOREFRONT = "01M1CNPKTV0018H2M1CXSTORE1";
const LEDGER = "01M1CNPKTV0018H2M1CXLEDGR1";

function build(): Scenario {
  const base = manifesting({
    kitInventory: INVENTORY,
    kitServers: KIT_SERVERS,
  });
  return {
    ...base,
    name: "token-usage-settings",
    says: "Kit › Skills with a list per skill of the repositories whose Jobs get it, Fleet › Token use, and Fleet › Repository with connectors per repository, armada's off from its armada.yml",
    state: {
      ...base.state,
      holds: {
        ...base.state.holds,
        repositories: [
          // armada's armada.yml says `connectors: false`, so its switch starts off.
          ...(base.state.holds.repositories ?? []).map((one) =>
            one.manifest?.id === MANIFEST_ID
              ? {
                  ...one,
                  manifest: {
                    ...one.manifest,
                    connectors_off: true,
                  },
                }
              : one,
          ),
          ANOTHER(STOREFRONT, "storefront"),
          ANOTHER(LEDGER, "ledger"),
        ],
      },
      settings: {
        ...MOCK_SETTINGS,
        settings: [
          ...MOCK_SETTINGS.settings,
          // ledger has pptx switched off, and no repository has a connectors switch of its own set.
          ...tokenUseSettings(
            {},
            {
              [LEDGER]: [
                "office-skills:pptx",
              ],
            },
          ),
        ],
      },
    },
  };
}

export const tokenUsageSettings: Scenario = build();
