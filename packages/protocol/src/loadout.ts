// A session's Loadout, mirrored by hand from `crates/ipc/src/loadout.rs`: what its agent knows, was
// told and used each turn, and where each came from. Names, sources and counts only.

/** Where an item came from. `kit` is the person's harness home; `managed` is set by an organisation. */
export type LoadoutSource = "kit" | "armada" | "repository" | "plugin" | "mcp" | "built_in" | "managed";

/** What an item is. The first five are what the agent knows; the last three what it was told. */
export type LoadoutKind =
  | "skill"
  | "subagent"
  | "mcp_server"
  | "tool"
  | "memory_file"
  | "output_style"
  | "rules"
  | "injection";

/** An MCP server is `connected`, `failed` or `pending`; a tool `deferred` or `loaded`. */
export type LoadoutState = "connected" | "failed" | "pending" | "deferred" | "loaded";

/** One thing the agent knows or was told. */
export type LoadoutItem = {
  kind: LoadoutKind;
  name: string;
  source: LoadoutSource;
  /** The plugin or MCP server it came from. */
  from?: string;
  state?: LoadoutState;
  /** Where the engine's breakdown gives one. */
  tokens?: number;
};

/** The whole list, as the `equipped` fact carries it. */
export type LoadoutItems = { items: LoadoutItem[] };

export type LoadoutUseKind = "skill" | "tool" | "subagent";

/** One use in a turn: a skill called, a tool loaded or called, a subagent dispatched. */
export type LoadoutUse = {
  /** When the turn it was made in began. */
  turn_started_at: string;
  kind: LoadoutUseKind;
  name: string;
  source: LoadoutSource;
  from?: string;
  /** The model a subagent was dispatched on, where the dispatch named one. */
  model?: string;
  /** The subagent that made the call. Absent for the session's own agent. */
  by?: string;
};

/** The uses of one turn, in the order they were reported. A use repeated in its turn is one. */
export type LoadoutTurn = { started_at: string; used: LoadoutUse[] };

/** On every `SessionRecord`, for the header mark and its hover card. */
export type LoadoutSummary = {
  knows: number;
  told: number;
  /** Absent until a use is reported. */
  latest_turn?: LoadoutTurn;
};

/** `get_session_loadout` (`GET /sessions/loadout?session_id=`): the whole Loadout, for the overlay. */
export type SessionLoadout = {
  items: LoadoutItem[];
  /** Oldest first, at most the last fifty. */
  turns: LoadoutTurn[];
};
