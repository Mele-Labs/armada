// The setup a person already works with, as the mock's own machine holds it —
// #1491. Its own file rather than a block in `manifest-fleet.ts`, which is at
// its length already.
//
// **No vendor here.** The harness names itself over the wire, and the mock is
// not the adapter.

import type { KitAllowedCommand, KitInventory, SetupItem } from "@armada/protocol";

/** Where Fleet spells each source of an allowlist command in the inventory. */
function sourceSpelled(one: KitAllowedCommand): string {
  switch (one.source) {
    case "retro_item":
      return `retro item ${one.lesson_id ?? ""}`.trim();
    case "always_allow":
      return "always allow";
    case "by_hand":
      return "written by hand";
  }
}

/** The allowlist kind as Fleet reads it, each command named as it was written and its source spelled as Fleet spells it. */
export function allowlistRead(items: readonly SetupItem[]): KitInventory {
  return withAllowlist(KIT_INVENTORY, items);
}

/** `inventory` with its allowlist kind read as `items`. */
export function withAllowlist(inventory: KitInventory, items: readonly SetupItem[]): KitInventory {
  return {
    ...inventory,
    kinds: inventory.kinds.map((one) =>
      one.kind === "allowlist" ? { kind: "allowlist", read: { what: "read", items: [...items], unreadable: [] } } : one,
    ),
  };
}

/** The allowlist as the commands `remove_kit_allowed_command` answers with become its rows. */
export function withAllowed(inventory: KitInventory, allowed: readonly KitAllowedCommand[]): KitInventory {
  return withAllowlist(
    inventory,
    allowed.map((one) => ({ name: one.run, source: sourceSpelled(one) })),
  );
}


/** What a person on this mock machine already has. */
export const KIT_INVENTORY: KitInventory = {
  harness: "An agent CLI",
  home: "/Users/user/.agent",
  present: true,
  kinds: [
    {
      kind: "skills",
      read: {
        what: "read",
        items: [
          {
            name: "humanizer",
            says: "Rewrite AI-sounding text so it reads like the writer without changing what it says.",
            source: "/Users/user/.agent/skills/humanizer",
          },
          {
            name: "impact-analysis",
            says: "Use when the user wants to know what will break if they change something.",
            source: "/Users/user/.agent/skills/impact-analysis",
          },
        ],
        unreadable: [
          {
            source: "/Users/user/.agent/skills/half/SKILL.md",
            why: "front matter opens and never closes",
          },
        ],
      },
    },
    {
      kind: "plugins",
      read: {
        what: "read",
        items: [
          {
            name: "code-simplifier@official",
            says: "version 1.0.0",
            source: "/Users/user/.agent/plugins/cache/code-simplifier/1.0.0",
          },
        ],
        unreadable: [],
      },
    },
    {
      kind: "agent_file",
      read: {
        what: "read",
        items: [
          {
            name: "AGENTS.md",
            says: "21 lines, opening # Global Instructions",
            source: "/Users/user/.agent/AGENTS.md",
          },
        ],
        unreadable: [],
      },
    },
    { kind: "sub_agents", read: { what: "read", items: [], unreadable: [] } },
    { kind: "commands", read: { what: "read", items: [], unreadable: [] } },
    {
      kind: "mcp_servers",
      read: {
        what: "read",
        items: [
          {
            name: "gitnexus",
            says: "gitnexus-mcp",
            source: "/Users/user/.agent.json",
          },
        ],
        unreadable: [],
      },
    },
    // Read since 23.35, from `~/.armada/allowed-commands`. Nothing in it until a command is added.
    { kind: "allowlist", read: { what: "read", items: [], unreadable: [] } },
    {
      kind: "models",
      read: {
        what: "not_read",
        why: "which models a Job may use is resolved against a Manifest, and that is #41",
      },
    },
  ],
};

/** An allowlist file that is there and will not read: the kind says so, with why, and never draws as empty. */
export const NOT_READ_ALLOWLIST: KitInventory = {
  ...KIT_INVENTORY,
  kinds: KIT_INVENTORY.kinds.map((one) =>
    one.kind === "allowlist"
      ? { kind: "allowlist", read: { what: "not_read", why: "allowed-commands would not read: line 3 is not a command" } }
      : one,
  ),
};

