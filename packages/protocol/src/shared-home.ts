// Kit against the shared home, mirrored by hand from `crates/ipc/src/shared_home.rs`.
// `docs/concepts/kit.md`, *The shared home*.

/** A kind the home carries, in the order What syncs lists them. */
export type HomeKind =
  | "skills"
  | "plugins"
  | "global_instructions"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "workflows"
  | "memory"
  | "mods";

/** How a row stands against the home: `here` is only on this machine, `home` only in the home. */
export type HomeRowState = "same" | "differs" | "here" | "home";

/** One Kit row against the home. A row that differs carries both versions, `here` and `there`. */
export type HomeRow = {
  kind: HomeKind;
  name: string;
  state: HomeRowState;
  says?: string;
  /** Where it comes from: a memory note's repository, an allowlist line's origin. */
  source?: string;
  /** A memory note's repository, by its folder's name, which names the row in an act. */
  repository?: string;
  /** On an MCP server alone: `false` where this machine has no server of that name, so its key is not here. */
  key_here?: boolean;
  here?: string;
  there?: string;
};

/** Something of a kind that would not read, named rather than left out. */
export type HomeUnreadable = { kind: HomeKind; at: string; why: string };

/** The home this machine holds a copy of: the login it belongs to and the repository's name. */
export type HomeNamed = { account: string; repository: string };

/** `GET /shared_home`, and what every act on it answers with. `home` is left out where this machine holds no copy yet. */
export type SharedHomeRead = { home?: HomeNamed; rows: HomeRow[]; unreadable: HomeUnreadable[] };

/** `POST /shared_home/add` and `POST /shared_home/bring`: which row. `repository` on a memory note alone. */
export type HomeRowAt = { kind: HomeKind; name: string; repository?: string };

/** Which side of a row that differs is kept: `here` goes into the home, `there` replaces this machine's. */
export type HomeSide = "here" | "there";

/** `POST /shared_home/choose`. */
export type ChooseHomeSide = { row: HomeRowAt; side: HomeSide };

/** One kind's switch in What syncs, and `POST /shared_home/kinds/set`'s body. */
export type HomeKindSwitch = { kind: HomeKind; on: boolean };

/** `GET /shared_home/kinds`: every kind, in order, off until it is turned on. */
export type HomeKinds = { kinds: HomeKindSwitch[] };
