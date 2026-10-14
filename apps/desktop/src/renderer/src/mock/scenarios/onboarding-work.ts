// The work machine, first run: signed out. Signing in with the work account sets up its own identity,
// store and shared home, and nothing of the personal side shows. The personal login is added on
// purpose from the account chip, and every kind it could sync starts off. The walk `onboarding-work`
// plays it.

import type { KitInventory } from "@armada/protocol";
import type { SharedRow } from "@armada/components";

import { KIT_INVENTORY, manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";
import { PERSONAL_ITEMS, withPlanUsage } from "../onboarding-items";

const SOURCE = "/Users/user/.agent/skills";

const INVENTORY: KitInventory = {
  ...KIT_INVENTORY,
  kinds: KIT_INVENTORY.kinds.map((one) => {
    if (one.kind === "skills") {
      return {
        kind: "skills",
        read: {
          what: "read",
          items: [
            { name: "code-review", source: `${SOURCE}/code-review` },
            { name: "release-notes", source: `${SOURCE}/release-notes` },
          ],
          unreadable: [],
        },
      };
    }
    if (one.kind === "plugins") return { kind: "plugins", read: { what: "read", items: [], unreadable: [] } };
    if (one.kind === "mcp_servers") {
      return { kind: "mcp_servers", read: { what: "read", items: [{ name: "linear", says: "mcp.linear.app", source: "/Users/user/.agent.json" }], unreadable: [] } };
    }
    return one;
  }),
};

const row = (kind: string, name: string, more: Partial<SharedRow> = {}): SharedRow => ({ kind, name, state: "same", ...more });

const ROWS: SharedRow[] = [
  row("skills", "code-review"),
  row("skills", "release-notes"),
  row("global_instructions", "AGENTS.md", { source: "/Users/user/.agent/AGENTS.md" }),
  row("workflows", "deploy"),
  row("mcp_servers", "linear"),
  row("memory", "Release checklist", { source: "api", says: "Check the changelog before the tag" }),
];

function build(): Scenario {
  const base = manifesting({ kitInventory: INVENTORY });
  return {
    ...base,
    name: "onboarding-work",
    says: "The work machine on first run: the work account signs in alone, and the personal login is added on purpose",
    behaves: withPlanUsage(base.behaves),
    draft: {
      ...(base.draft ?? {}),
      sharedHome: { account: "mele-work", repository: "armada-home", rows: ROWS },
      account: {
        signIn: { login: "mele-work", code: "RTHV-2CLM", repository: "armada-home" },
        add: { login: "nickmele", code: "WDJB-MJHT", items: PERSONAL_ITEMS },
      },
    },
  };
}

export const onboardingWork: Scenario = build();
