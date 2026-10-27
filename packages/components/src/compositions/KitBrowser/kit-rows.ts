import { kitSourcePath } from "@armada/protocol";
import type { KitPieceKind } from "@armada/protocol";

import { linesOf, sourceWords, styleFrom } from "../KitSetup/KitSetup";
import type { KitSetupItem, KitSetupKindWord, KitSetupRead } from "../KitSetup/KitSetup";
import type { SharedHome, SharedRow, HomeSyncItem } from "../KitSetup/SharedHome";
import type { KitServerRowProps } from "../KitServers/KitServers";

/** The kinds Kit draws as a list with the picked item beside it, by the id their sidebar entry has. */
export type KitKindId = "skills" | "plugins" | "sub-agents" | "commands" | "servers" | "instructions" | "output-styles" | "allowlist" | "mods";

export const KIT_KINDS: readonly { id: KitKindId; label: string }[] = [
  { id: "skills", label: "Skills" },
  { id: "plugins", label: "Plugins" },
  { id: "sub-agents", label: "Sub agents" },
  { id: "commands", label: "Commands" },
  { id: "servers", label: "Servers" },
  { id: "instructions", label: "Instructions" },
  { id: "output-styles", label: "Output styles" },
  { id: "allowlist", label: "Allowlist" },
  { id: "mods", label: "Mods" },
];

/** What Fleet calls each kind. Instructions is the agent file, and Servers the ones connected outside Armada. */
const WORD: Record<KitKindId, KitSetupKindWord> = {
  skills: "skills",
  plugins: "plugins",
  "sub-agents": "sub_agents",
  commands: "commands",
  servers: "mcp_servers",
  instructions: "agent_file",
  "output-styles": "output_styles",
  allowlist: "allowlist",
  mods: "mods",
};

/** What the shared home calls each kind it carries. Output styles it does not, and Instructions are the global ones. */
export const HOME_KIND: Partial<Record<KitKindId, string>> = {
  skills: "skills",
  plugins: "plugins",
  "sub-agents": "sub_agents",
  commands: "commands",
  servers: "mcp_servers",
  instructions: "global_instructions",
  allowlist: "allowlist",
  mods: "mods",
};

const HOMED: ReadonlySet<string> = new Set(Object.values(HOME_KIND));

/** A kind's word for one of its items, as a plugin's list of what it brings names it. */
const ONE: Partial<Record<KitKindId, string>> = {
  skills: "Skill",
  "sub-agents": "Sub agent",
  commands: "Command",
  "output-styles": "Output style",
};

/** The kinds a plugin brings items of. */
const BROUGHT: readonly KitKindId[] = ["skills", "sub-agents", "commands", "output-styles"];

/** Another item a row leads to. */
/** `opens` is the file the item's own detail shows, absent where it has none. */
export type KitRef = { kind: KitKindId; id: string; name: string; word: string; opens?: string };

/** One row of a kind's list, and everything its detail draws. */
export type KitRow = {
  id: string;
  name: string;
  /** Where it comes from, in words: `Yours`, a plugin's name, `This repository`, `Armada`. */
  mark?: string;
  /** One plain fact on the row's right: a version, a line count, where an allowlist command came from. */
  meta?: string;
  /** A plugin's marketplace, which its name is listed without. */
  marketplace?: string;
  /** What the meta is, for its tooltip. */
  metaSaid?: string;
  /** Machine text, set in mono: a command, a program. */
  mono?: boolean;
  path?: string;
  /** The one file Open shows: the item's own, or a skill folder's `SKILL.md`. Absent for a folder with no single file, a plugin. */
  opens?: string;
  says?: string;
  /** Instructions: the file's opening heading. */
  heading?: string;
  /** An output style in force. */
  active?: boolean;
  /** Applied on top of whatever else is chosen, so set apart at the end. */
  always?: boolean;
  /** The allowlist's act. */
  removable?: boolean;
  /** A skill or plugin Remove takes out, by its name as the inventory spells it. */
  piece?: { kind: KitPieceKind; name: string; uninstall?: string };
  /** The plugin a skill comes with, which is what removes it. */
  from?: string;
  brings?: KitRef[];
  sync?: SharedRow | undefined;
  /** The item as What syncs names it, where the home carries its kind. */
  home?: HomeSyncItem;
  /** A mod: what it changes, and whether Armada ships it. */
  mod?: { kind: "theme" | "layout"; builtIn: boolean };
  /** A server connected outside Armada, or Kit's own. */
  server?: { tier: "outside" } | { tier: "kit"; row: KitServerRowProps };
};

export type KitRead = { rows: KitRow[]; notRead?: string };

/** Whether `words` holds every word of `query`, ignoring case. */
export function holds(words: string, query: string): boolean {
  const said = words.toLowerCase();
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter((one) => one !== "")
    .every((one) => said.includes(one));
}

function markOf(item: KitSetupItem, home: string): string {
  if (item.plugin !== undefined) return item.plugin;
  if (item.style !== undefined) return styleFrom(item.style);
  if (item.source === "" || item.source.startsWith(`${home}/`) || item.source === home) return "Yours";
  if (item.source === "Armada") return "Armada";
  return "This repository";
}

const providerOf = (plugin: string): string => plugin.split("@")[0] ?? plugin;

function rowOf(kind: KitKindId, item: KitSetupItem, sync: SharedRow | undefined, home: string): KitRow {
  const path = kitSourcePath(item.source, home);
  const base: KitRow = {
    id: `${kind}:${item.source}:${item.name}`,
    name: item.name,
    ...(path === null ? {} : { path }),
    ...(path === null || kind === "plugins" ? {} : { opens: kind === "skills" ? `${path}/SKILL.md` : path }),
    sync,
  };
  if (kind === "allowlist") {
    const { path: _path, opens: _opens, ...rest } = base;
    return { ...rest, mono: true, meta: sourceWords(item.source), removable: true };
  }
  if (kind === "mods") {
    // A mod Armada ships is on every machine already, so it has no standing against the home.
    const builtIn = item.mod?.builtIn === true;
    return { ...base, ...(builtIn ? { sync: undefined } : {}), mod: { kind: item.mod?.kind ?? "theme", builtIn } };
  }
  if (kind === "plugins") {
    const version = /^version (.+)$/.exec(item.says ?? "")?.[1];
    // Listed by its plain name; where it was installed from is a fact of its own, and the mark would say the same on every row.
    const [plain = item.name, marketplace] = item.name.split("@");
    return {
      ...base,
      name: plain,
      piece: { kind: "plugin", name: item.name, ...(item.uninstall === undefined ? {} : { uninstall: item.uninstall }) },
      ...(marketplace === undefined ? {} : { marketplace }),
      ...(version === undefined ? {} : { meta: version, metaSaid: "Version" }),
    };
  }
  const mark = markOf(item, home);
  if (kind === "instructions") {
    const parsed = /^(\d+) lines?, opening (.+)$/.exec(item.says ?? "");
    return {
      ...base,
      mark,
      ...(parsed === null ? (item.says === undefined ? {} : { says: item.says }) : { meta: `${parsed[1]} lines`, heading: parsed[2]! }),
    };
  }
  if (kind === "servers") return { ...base, mark: "Outside Armada", ...(item.says === undefined ? {} : { says: item.says }), server: { tier: "outside" } };
  const owned = kind !== "skills" ? {} : item.plugin !== undefined ? { from: item.plugin } : path === null ? {} : { piece: { kind: "skill" as const, name: item.name } };
  return {
    ...base,
    ...owned,
    mark,
    ...(item.says === undefined ? {} : { says: item.says }),
    ...(item.style?.applied === "active" ? { active: true } : {}),
    ...(item.style?.applied === "always" ? { always: true } : {}),
  };
}

/** One kind's rows off the read, the home's rows that are only there after them. */
export function rowsOf(kind: KitKindId, setup: KitSetupRead | undefined, shared: SharedHome | undefined, kitServers?: readonly KitServerRowProps[]): KitRead {
  if (setup === undefined) return { rows: [] };
  const words = kind === "instructions" ? (["agent_file", "global_instructions"] as const) : ([WORD[kind]] as const);
  const rows: KitRow[] = [];
  let notRead: string | undefined;
  for (const word of words) {
    const found = setup.kinds.find((one) => one.kind === word);
    const items = found?.read.what === "read" ? found.read.items : [];
    if (found?.read.what === "not_read") notRead = found.read.why;
    // A kind the home alone carries has nothing read and is not "not read".
    for (const { item, row } of linesOf(word, items, shared)) {
      const one = rowOf(kind, item, row, setup.home);
      // A mod Armada ships is on every machine already, so it is not one to sync.
      if (HOMED.has(word) && one.mod?.builtIn !== true) one.home = { kind: word, name: item.name };
      rows.push(one);
    }
  }
  if (kind === "instructions") {
    // The home's copy of a file this machine has is the same row, not a second one.
    // A row with the same id is the same file read twice, once as the agent's file and once as the home's kind.
    for (const apart of rows.filter((one) => one.sync !== undefined)) {
      const own = rows.find((one) => one !== apart && one.sync === undefined && (one.id === apart.id || (apart.path === undefined && one.name === apart.name)));
      if (own === undefined) continue;
      own.sync = apart.sync;
      if (apart.home !== undefined) own.home = apart.home;
      rows.splice(rows.indexOf(apart), 1);
    }
  }
  if (kind === "plugins") {
    for (const one of rows) {
      const provider = providerOf(one.name);
      one.brings = BROUGHT.flatMap((brought) =>
        rowsOf(brought, setup, undefined).rows
          .filter((row) => row.mark === provider)
          .map((row): KitRef => ({ kind: brought, id: row.id, name: row.name, word: ONE[brought] ?? "", ...(row.opens === undefined ? {} : { opens: row.opens }) })),
      );
    }
  }
  if (kind === "servers") {
    for (const server of kitServers ?? []) {
      rows.push({ id: `servers:kit:${server.name}`, name: server.name, mark: "Kit", says: server.address, server: { tier: "kit", row: server }, home: { kind: "mcp_servers", name: server.name } });
    }
  }
  // What is applied on top of the rest stands apart at the end.
  const ordered = [...rows.filter((one) => one.always !== true), ...rows.filter((one) => one.always === true)];
  return notRead === undefined ? { rows: ordered } : { rows: ordered, notRead };
}

/** Every name a kind holds, for a search over the sidebar. */
export const namesOf = (kind: KitKindId, setup: KitSetupRead | undefined, kitServers?: readonly KitServerRowProps[]): string[] =>
  rowsOf(kind, setup, undefined, kitServers).rows.map((one) => one.name);
