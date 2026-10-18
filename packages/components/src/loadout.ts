// What a Session's Loadout says to a person: where each item came from, what kind it is, and the order sources
// are offered in. The wire is `packages/protocol/src/loadout.ts`.

import type { LoadoutItem, LoadoutKind, LoadoutSource, LoadoutUse, LoadoutUseKind } from "@armada/protocol";

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
  return (one.source === "plugin" || one.source === "mcp") && one.from !== undefined && one.from !== "" ? one.from : SOURCE_SAID[one.source];
}

const SOURCE_ORDER: readonly LoadoutSource[] = ["kit", "armada", "repository", "plugin", "mcp", "built_in", "managed"];

/** The sources the items and uses name, each once, Kit first and plugins and servers by name inside theirs. */
export function sourcesOf(items: readonly LoadoutItem[], uses: readonly LoadoutUse[]): string[] {
  const all = [...items, ...uses];
  return SOURCE_ORDER.flatMap((source) => [...new Set(all.filter((one) => one.source === source).map(sourceSaid))].sort((a, b) => a.localeCompare(b)));
}

/** What the agent knows, in the order the panel lists them. */
export const KNOWN: readonly LoadoutKind[] = ["skill", "subagent", "mcp_server", "tool", "memory_file"];
/** What it was told. */
export const TOLD: readonly LoadoutKind[] = ["output_style", "rules", "injection"];

/** A kind as the heading over its items. */
export const KIND_SAID: Record<LoadoutKind, string> = {
  skill: "Skills",
  subagent: "Subagents",
  mcp_server: "MCP servers",
  tool: "Tools",
  memory_file: "Memory files",
  output_style: "Output style",
  rules: "Job rules",
  injection: "Injections",
};

/** One use's kind, beside its name. */
export const USE_SAID: Record<LoadoutUseKind, string> = { skill: "Skill", tool: "Tool", subagent: "Subagent" };

/** Where a use came from, and the model a subagent was sent on: `Armada · sonnet`. */
export function useSaid(use: LoadoutUse): string {
  return [sourceSaid(use), use.model].filter((one) => one !== undefined && one !== "").join(" · ");
}

/** How often a use was heard in its turn, `× 40`, and nothing where it was once. */
export function countSaid(use: LoadoutUse): string | undefined {
  return use.count !== undefined && use.count > 1 ? `× ${use.count}` : undefined;
}

/** The thread's own clock, to the minute, so a turn and the messages it wrote read as one clock. */
const CLOCK = new Intl.DateTimeFormat([], { hour: "numeric", minute: "2-digit", hour12: false });

/** When a turn began, as the thread's clock reads it: `14:02`. The wire's own words for a stamp that will not parse. */
export function turnSaid(at: string): string {
  const when = Date.parse(at);
  return Number.isNaN(when) ? at : CLOCK.format(when);
}

/** One Loadout row, from Knows, Told or a turn's uses: a use's `skill`, `tool` or `subagent` is the item of that kind. */
export type LoadoutRef = { kind: LoadoutKind; name: string; source: LoadoutSource; from?: string };

/** The item a row stands for, without what only Knows or a use carries. */
export function loadoutRefOf(one: LoadoutRef): LoadoutRef {
  return { kind: one.kind, name: one.name, source: one.source, ...(one.from === undefined ? {} : { from: one.from }) };
}

/** One identity for an item wherever it is listed, so a use and the item it used are the same row. */
export function loadoutKeyOf(one: LoadoutRef): string {
  return `${one.kind}:${one.source}:${one.from ?? ""}:${one.name}`;
}

/** The tools an MCP server gave, in the order the Loadout lists them: its own by `from`, or by the `mcp__<server>__` prefix. */
export function toolsOf(server: LoadoutRef, items: readonly LoadoutItem[]): LoadoutItem[] {
  const named = server.from ?? server.name;
  const prefix = `mcp__${server.name}__`;
  return items.filter((one) => one.kind === "tool" && (one.from === named || one.name.startsWith(prefix)));
}
