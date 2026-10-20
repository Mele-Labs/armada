import { linesOf, sourceWords, styleFrom } from "../KitSetup/KitSetup";
import type { KitSetupItem, KitSetupKindWord, KitSetupRead } from "../KitSetup/KitSetup";
import type { SharedHome, SharedRow } from "../KitSetup/SharedHome";
import type { KitServerRowProps } from "../KitServers/KitServers";

/** The kinds Kit draws as a list with the picked item beside it, by the id their sidebar entry has. */
export type KitKindId = "skills" | "plugins" | "sub-agents" | "commands" | "servers" | "instructions" | "output-styles" | "allowlist";

export const KIT_KINDS: readonly { id: KitKindId; label: string }[] = [
  { id: "skills", label: "Skills" },
  { id: "plugins", label: "Plugins" },
  { id: "sub-agents", label: "Sub agents" },
  { id: "commands", label: "Commands" },
  { id: "servers", label: "Servers" },
  { id: "instructions", label: "Instructions" },
  { id: "output-styles", label: "Output styles" },
  { id: "allowlist", label: "Allowlist" },
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
};

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
export type KitRef = { kind: KitKindId; id: string; name: string; word: string };

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
  says?: string;
  /** Instructions: the file's opening heading. */
  heading?: string;
  /** An output style in force. */
  active?: boolean;
  /** Applied on top of whatever else is chosen, so set apart at the end. */
  always?: boolean;
  /** The allowlist's act. */
  removable?: boolean;
  brings?: KitRef[];
  sync?: SharedRow | undefined;
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
  const base: KitRow = {
    id: `${kind}:${item.source}:${item.name}`,
    name: item.name,
    ...(item.source === "" || item.source === "Armada" || item.source.startsWith("built in") ? {} : { path: item.source }),
    sync,
  };
  if (kind === "allowlist") {
    const { path: _path, ...rest } = base;
    return { ...rest, mono: true, meta: sourceWords(item.source), removable: true };
  }
  if (kind === "plugins") {
    const version = /^version (.+)$/.exec(item.says ?? "")?.[1];
    // Listed by its plain name; where it was installed from is a fact of its own, and the mark would say the same on every row.
    const [plain = item.name, marketplace] = item.name.split("@");
    return {
      ...base,
      name: plain,
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
  return {
    ...base,
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
    for (const { item, row } of linesOf(word, items, shared)) rows.push(rowOf(kind, item, row, setup.home));
  }
  if (kind === "instructions") {
    // The home's copy of a file this machine has is the same row, not a second one.
    for (const apart of rows.filter((one) => one.path === undefined && one.sync !== undefined)) {
      const own = rows.find((one) => one !== apart && one.name === apart.name && one.sync === undefined);
      if (own === undefined) continue;
      own.sync = apart.sync;
      rows.splice(rows.indexOf(apart), 1);
    }
  }
  if (kind === "plugins") {
    for (const one of rows) {
      const provider = providerOf(one.name);
      one.brings = BROUGHT.flatMap((brought) =>
        rowsOf(brought, setup, undefined).rows
          .filter((row) => row.mark === provider)
          .map((row): KitRef => ({ kind: brought, id: row.id, name: row.name, word: ONE[brought] ?? "" })),
      );
    }
  }
  if (kind === "servers") {
    for (const server of kitServers ?? []) {
      rows.push({ id: `servers:kit:${server.name}`, name: server.name, mark: "Kit", says: server.address, server: { tier: "kit", row: server } });
    }
  }
  // What is applied on top of the rest stands apart at the end.
  const ordered = [...rows.filter((one) => one.always !== true), ...rows.filter((one) => one.always === true)];
  return notRead === undefined ? { rows: ordered } : { rows: ordered, notRead };
}

/** Every name a kind holds, for a search over the sidebar. */
export const namesOf = (kind: KitKindId, setup: KitSetupRead | undefined, kitServers?: readonly KitServerRowProps[]): string[] =>
  rowsOf(kind, setup, undefined, kitServers).rows.map((one) => one.name);
