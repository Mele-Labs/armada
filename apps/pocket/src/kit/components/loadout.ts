// What a Session's Loadout says to a person: where each item came from, what kind it is, and the order sources
// are offered in. The wire is `crates/ipc/src/loadout.rs`.

import type { LoadoutUse, LoadoutSource } from "../../wire/index";

/** A plugin and an MCP server are named for themselves; the rest by where they came from. */
const SOURCE_SAID: Record<LoadoutSource, string> = {
  kit: "Kit",
  armada: "Armada",
  repository: "Repository",
  plugin: "Plugin",
  mcp: "MCP",
  built_in: "Built-in",
  managed: "Managed",
};

/** Where an item or a use came from, as a person reads it: `Kit`, `Armada`, the plugin's own name. */
export function sourceSaid(one: { source: LoadoutSource; from?: string }): string {
  return (one.source === "plugin" || one.source === "mcp") &&
    one.from !== undefined &&
    one.from !== ""
    ? one.from
    : SOURCE_SAID[one.source];
}

/** Where a use came from, and the model a subagent was sent on: `Armada · sonnet`. */
export function useSaid(use: LoadoutUse): string {
  return [
    sourceSaid(use),
    use.model,
  ]
    .filter((one) => one !== undefined && one !== "")
    .join(" · ");
}

/** How often a use was heard in its turn, `× 40`, and nothing where it was once. */
export function countSaid(use: LoadoutUse): string | undefined {
  return use.count !== undefined && use.count > 1 ? `× ${use.count}` : undefined;
}

/** The prefix an MCP server's tools carry in their names. */
const mcpPrefix = (server: string) => `mcp__${server}__`;

/** A use's tool as a person reads it: an MCP tool without the `mcp__<server>__` its source already says. */
export function toolSaid(use: LoadoutUse): string {
  const prefix = use.source === "mcp" && use.from !== undefined ? mcpPrefix(use.from) : undefined;
  return prefix !== undefined && use.name.startsWith(prefix) && use.name.length > prefix.length
    ? use.name.slice(prefix.length)
    : use.name;
}
