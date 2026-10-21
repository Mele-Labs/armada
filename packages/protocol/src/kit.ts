// Kit's MCP servers, and each manifest's word over one. Since protocol 15.2.
// `crates/ipc/src/kit.rs`, #1275.
//
// `resolves` crosses rather than being worked out here: it is fleet's own
// resolution over the two tiers, and the same answer decides the document a
// drone is spawned against. A surface that recomputed it could draw a server
// as reaching a drone that no drone is handed.

/**
 * Where a server is, in the two shapes the agent CLI's schema accepts. Tagged
 * by `transport`, so a reader matches one field rather than guessing from
 * which of two optional keys turned up.
 */
export type ServerAddress =
  | { transport: "stdio"; command: string; args: string[] }
  | { transport: "http"; url: string };

/** Kit's own tier for a server: whether a drone gets it where the manifest is silent. */
export type ReachesDrones = "no" | "yes";

/**
 * What one manifest says about one of kit's servers. Absent where it has said
 * nothing, which is kit's default answering.
 */
export type ManifestReach = "extended" | "restricted";

/** One row of `get_kit_servers`. */
export type KitServerRow = {
  name: string;
  address: ServerAddress;
  /** Kit's own default, across every manifest. */
  drones: ReachesDrones;
  /** This manifest's word, absent where it has none. */
  manifest?: ManifestReach;
  /** Whether a drone dispatched here gets it. The resolution, not a hint. */
  resolves: boolean;
  /** When it was added, by fleet's clock. */
  added_at: string;
  /** Who added it, in the envelope's spelling. Left as `string` like `actor`. */
  by: string;
};

/** `get_kit_servers`' answer, and what every act on kit answers with. */
export type KitServers = {
  servers: KitServerRow[];
};

/** The body of `add_kit_server`. A name kit holds replaces that server's address. */
export type AddKitServer = {
  name: string;
  address: ServerAddress;
};

/** The body of `forget_kit_server`. A name kit does not hold is a 409. */
export type ForgetKitServer = {
  name: string;
};

/** The body of `set_kit_server_reach` — kit's own tier. */
export type SetKitServerReach = {
  name: string;
  drones: ReachesDrones;
};

/**
 * The body of `set_manifest_server_reach` — the manifest's own tier.
 *
 * **`null` is sent, never implied.** Fleet refuses a body with no `reach` key
 * rather than reading it as a take-back, `SetModel`'s reason: a bridge that
 * dropped the field would throw away a person's word about who may reach a
 * server and get a 200 for it.
 */
export type SetManifestServerReach = {
  name: string;
  reach: ManifestReach | null;
};

// The setup a person already works with, read from their agent harness's own
// home and shown by kind. Since protocol 17.2; `output_styles` and `style` added with #2413. `crates/ipc/src/kit.rs`, #1491.
//
// **No credential on any of it.** A connected server crosses as the program's
// own file name or the host it is at, and never as what follows either — no
// argument list, no query string, no userinfo, no environment. What crosses
// could not start the server it names, and allowing one stays its own act on a
// kit row a person added themselves.

/** One thing a person already has. */
export type SetupItem = {
  name: string;
  /** Its own words for itself — or, for a connected server, what it is at. */
  says?: string;
  /** Where it came from, as a person would type it. */
  source: string;
  /** Output styles only. Absent on every other kind's rows. */
  style?: OutputStyleFacts;
  /** The plugin that brought it, without its marketplace. Absent on the person's own. */
  plugin?: string;
  /** Plugins only: the command Remove runs, as Fleet spells it for the harness. Not on the wire yet. */
  uninstall?: string;
};

/** Where an output style came from. Armada's word, never a harness's. */
export type OutputStyleFrom = "person" | "repository" | "plugin" | "built_in" | "armada";

/**
 * Whether an output style is in force. `active` is chosen in the settings
 * files; `always` is on top of the active one, whichever it is.
 */
export type OutputStyleApplied = "active" | "available" | "always";

/** An output style's own facts. **Never its body.** */
export type OutputStyleFacts = {
  from: OutputStyleFrom;
  applied: OutputStyleApplied;
  /** The plugin's own name, on a `plugin` style only. */
  plugin?: string;
};

/**
 * The file or folder an item's `source` names, or `null` where it names none: empty, `Armada`, or
 * the harness's own `built in to …`. A path not already absolute is read under the harness home.
 * Bridge draws Open in editor and Show in Finder off this, and main checks a request against it.
 */
export function kitSourcePath(source: string, home: string): string | null {
  if (source === "" || source === "Armada" || source.startsWith("built in")) return null;
  return source.startsWith("/") ? source : `${home.replace(/\/+$/, "")}/${source}`;
}

/** Something of the right shape in the right place that would not read. */
export type SetupUnreadable = {
  source: string;
  why: string;
};

/**
 * What became of one kind. **Tagged**, so an empty list is *you have none of
 * these* and never *nothing looked*.
 */
export type WhatWasRead =
  | { what: "read"; items: SetupItem[]; unreadable: SetupUnreadable[] }
  | { what: "not_read"; why: string };

/** Armada's word for one kind of thing, never a harness's. */
export type SetupKind =
  | "skills"
  | "plugins"
  | "agent_file"
  | "output_styles"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models";

/** One kind, and what became of it. */
export type SetupKindRow = {
  kind: SetupKind;
  read: WhatWasRead;
};

/** `get_kit_inventory`'s answer. */
export type KitInventory = {
  /**
   * The harness, in its own name. **Drawn, never matched on** — it arrives as
   * data an adapter produced, which is what lets a second harness draw here
   * without this file changing.
   */
  harness: string;
  home: string;
  /** Whether that home is there at all. */
  present: boolean;
  /** Every kind, in a fixed order, including the ones nothing reads yet. */
  kinds: SetupKindRow[];
};

/**
 * Where a command in Kit's allowlist came from. Since protocol 23.35.
 * `lesson_id` rides with `retro_item`.
 */
export type KitAllowedSource = "retro_item" | "always_allow" | "by_hand";

/** One command Kit's allowlist holds. `run` is what `remove_kit_allowed_command` names. */
export type KitAllowedCommand = {
  run: string;
  source: KitAllowedSource;
  lesson_id?: string;
};

/** `remove_kit_allowed_command`'s answer: what the allowlist holds now. Since protocol 23.35. */
export type KitAllowedCommands = {
  commands: KitAllowedCommand[];
};

/** The body of `remove_kit_allowed_command`: `POST /kit/allowed_commands/remove`. A line spelled otherwise is a 409. */
export type RemoveKitAllowedCommand = {
  run: string;
};

/** What Add installs from a source: a plugin through its marketplace, or a skill on its own. Not on the wire yet. */
export type KitPieceKind = "plugin" | "skill";

/** The body of `look_at_kit_source`: what the person pasted, `owner/repository`, a GitHub URL or a website's. */
export type LookAtKitSource = {
  source: string;
};

/**
 * `look_at_kit_source`'s answer: what Fleet found the source to be, before anything runs. `repository` is where it
 * resolved to, `owner/repository`, and `commands` are what Install runs, in order. A plugin's `name` and `marketplace`
 * are read from its marketplace file. `unknown` says why in Fleet's words.
 */
export type KitSourceFound =
  | { found: "plugin"; name: string; marketplace: string; repository: string; skills: string[]; commands: string[] }
  | { found: "skill"; name: string; repository: string; commands: string[] }
  | { found: "unknown"; why: string };

/** The body of `install_kit_piece`: what a look found, by its `repository` and kind. `manifest` also writes it into the person's Kit manifest. */
export type InstallKitPiece = {
  source: string;
  kind: KitPieceKind;
  manifest: boolean;
};

/**
 * `install_kit_piece`'s answer. **A command that exits non-zero is an answer, not a refusal**: Fleet ran it,
 * and what it printed is what the person reads. `command` is the one that failed, of the ones Add showed.
 */
export type KitPieceInstalled =
  | { state: "installed"; setup: KitInventory }
  | { state: "failed"; command: string; exit_code: number; error: string };

/** The body of `remove_kit_piece`. `name` is the item's name as the inventory spells it, `impeccable@impeccable` for a plugin. */
export type RemoveKitPiece = {
  kind: KitPieceKind;
  name: string;
};

/** `remove_kit_piece`'s answer. `trashed` is where a skill's folder went in the Trash, which `restore_kit_piece` names; `null` for a plugin, which is uninstalled. */
export type KitPieceRemoved = {
  setup: KitInventory;
  trashed: string | null;
};

/** The body of `restore_kit_piece`: a skill's folder back out of the Trash, by where `remove_kit_piece` put it. */
export type RestoreKitPiece = {
  trashed: string;
};
