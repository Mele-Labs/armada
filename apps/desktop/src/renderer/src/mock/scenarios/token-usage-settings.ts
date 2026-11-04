// Settings Fleet does not serve yet, drawn as schema rows: Fleet › Token use with four Machine settings, and Fleet ›
// Repository with the connectors switch saved off. Fleet's `get_settings` answers them here; the walk
// `tokenUsageSettings` plays it.

import { MOCK_SETTINGS, TOKEN_USE_SETTINGS } from "@armada/settings/fake";

import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";

function build(): Scenario {
  const base = manifesting();
  return {
    ...base,
    name: "token-usage-settings",
    says: "System with Fleet › Token use and Fleet › Repository, served as schema rows before Fleet has them",
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
