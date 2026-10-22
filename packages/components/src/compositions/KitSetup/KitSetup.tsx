import type { SharedHome, SharedRow } from "./SharedHome";

// The setup a person already works with, read from their agent harness's own home — #1491 — as the
// Kit page's list and detail take it (`../KitBrowser`). Types and the words for them; nothing is drawn here.

export type KitSetupRead = {
  /** The harness, in its own name. **Drawn, never matched on.** */
  harness: string;
  /** Where it was read, as a person would type it. */
  home: string;
  /** Whether that home is there at all. */
  present: boolean;
  /** Every kind, in the order fleet answered them. */
  kinds: KitSetupKind[];
};

/** Armada's word for one kind of thing, never a harness's. */
export type KitSetupKindWord =
  | "skills"
  | "plugins"
  | "agent_file"
  | "output_styles"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models"
  // The three the shared home carries that fleet's read does not: they are drawn only against a home.
  | "workflows"
  | "global_instructions"
  | "memory";

export type KitSetupKind = {
  kind: KitSetupKindWord;
  read:
    | { what: "read"; items: KitSetupItem[]; unreadable: KitSetupUnreadable[] }
    | { what: "not_read"; why: string };
};

export type KitSetupItem = {
  name: string;
  /** Its own words for itself — or, for a server, the program or host it is at. */
  says?: string;
  /** Where it came from. Drawn small, and never a control. */
  source: string;
  /** Output styles only: where the style came from and whether it is in force. */
  style?: KitSetupStyle;
  /** The plugin that provides it, by its own name. Absent for your own. */
  plugin?: string;
  /** A plugin's: the command Remove runs, as Fleet spells it. */
  uninstall?: string;
};

export type KitSetupStyle = {
  from: "person" | "repository" | "plugin" | "built_in" | "armada";
  applied: "active" | "available" | "always";
  /** The plugin's own name, on a `plugin` style. */
  plugin?: string;
};

export type KitSetupUnreadable = {
  source: string;
  why: string;
};

const KIND_LABEL: Record<KitSetupKindWord, string> = {
  skills: "Skills",
  plugins: "Plugins",
  agent_file: "Agent file",
  output_styles: "Output styles",
  sub_agents: "Sub agents",
  commands: "Commands",
  mcp_servers: "MCP servers",
  allowlist: "Allowlist",
  models: "Models",
  workflows: "Workflows",
  global_instructions: "Global instructions",
  memory: "Memory",
};

/** Kit's words for its kinds, for the pages that draw a kind by itself. */
export const KIT_KIND_LABEL = KIND_LABEL;

/** One row of a kind's list: the thing read, or the thing only the home has, and how it stands against the home. */
type Line = { item: KitSetupItem; row?: SharedRow | undefined };

/** What is read, then what only the home has. Where a kind has no home, every line is just what was read. */
export function linesOf(kind: KitSetupKindWord, items: KitSetupItem[], shared: SharedHome | undefined): Line[] {
  const rows = shared?.rows.filter((row) => row.kind === kind) ?? [];
  const read = items.map((item) => ({ item, row: rows.find((row) => row.name === item.name) }));
  const apart = rows
    .filter((row) => !items.some((item) => item.name === row.name))
    .map((row) => ({ item: { name: row.name, ...(row.says === undefined ? {} : { says: row.says }), source: row.source ?? "" }, row }));
  return [...read, ...apart];
}

/**
 * Where an allowlist command came from, in words. **Fleet's own spelling is
 * `retro item <id>`, `always allow` or `written by hand`**, and the id is what
 * the retro page is for, so only the kind is drawn. A spelling this build has
 * no words for is drawn as it came.
 */
export function sourceWords(source: string): string {
  if (source.startsWith("retro item")) return "Retro item";
  if (source === "always allow") return "Always allow";
  if (source === "written by hand") return "Written by hand";
  return source;
}

/** Where a style came from, in words. A plugin is named by its own name. */
export function styleFrom(style: KitSetupStyle): string {
  switch (style.from) {
    case "person":
      return "Yours";
    case "repository":
      return "This repository";
    case "plugin":
      return style.plugin ?? "A plugin";
    case "built_in":
      return "Built in";
    case "armada":
      return "Armada";
  }
}
