// Kit against the shared home, mirrored by hand from `crates/ipc/src/shared_home.rs`.
// `docs/concepts/kit.md`, *The shared home*.

import type { ModKind } from "./mods";

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
  /** On a mod alone: the kind its `mod.toml` names, this machine's where it has one and else the home's. */
  mod_kind?: ModKind;
  here?: string;
  there?: string;
};

/** Something of a kind that would not read, named rather than left out. */
export type HomeUnreadable = { kind: HomeKind; at: string; why: string };

/** Which profile's home: a Mac can hold two profiles, each with its own. `POST /shared_home/sync`'s body, and the `?login=` of `GET /shared_home` and `GET /shared_home/kinds`. */
export type HomeProfile = { login: string };

/** The home this machine holds a copy of: the login it belongs to and the repository's name. */
export type HomeNamed = { account: string; repository: string };

/** Where a home is or will be, served while this machine holds no copy. `account` is the owner, the profile's login or one of its organisations. `exists` is set where the forge already has it, made on another Mac: shown and not changed, and Sync clones it. */
export type HomePlanned = { account: string; repository: string; exists: boolean };

/** `GET /shared_home`, and what every act on it answers with. `home` is left out where this machine holds no copy yet, and `planned` is served then. */
export type SharedHomeRead = { home?: HomeNamed; planned?: HomePlanned; rows: HomeRow[]; unreadable: HomeUnreadable[] };

/** A row only on this machine that Sync did not add. `why` is a sentence naming the row and why it stayed: a host, never a credential. */
export type HomeLeft = { kind: HomeKind; name: string; repository?: string; why: string };

/** `POST /shared_home/sync`: the home read again, and each row only on this machine Sync left here. */
export type HomeSynced = SharedHomeRead & { left: HomeLeft[] };

/** `POST /shared_home/add` and `POST /shared_home/bring`: which row, in which profile's home. `repository` on a memory note alone. */
export type HomeRowAt = { login: string; kind: HomeKind; name: string; repository?: string };

/** Which side of a row that differs is kept: `here` goes into the home, `there` replaces this machine's. */
export type HomeSide = "here" | "there";

/** `POST /shared_home/choose`. */
export type ChooseHomeSide = { row: HomeRowAt; side: HomeSide };

/** One kind's switch in What syncs. */
export type HomeKindSwitch = { kind: HomeKind; on: boolean };

/** `POST /shared_home/kinds/set`'s body: one kind switched for one profile. */
export type SetHomeKind = { login: string; kind: HomeKind; on: boolean };

/** One item switched on in What syncs on its own, whatever its kind's switch says. `repository` on a memory note alone. */
export type HomeItem = { kind: HomeKind; name: string; repository?: string };

/** `POST /shared_home/items/set`'s body: one item switched for one profile. */
export type SetHomeItem = { login: string; kind: HomeKind; name: string; repository?: string; on: boolean };

/** `GET /shared_home/kinds`: one profile's every kind, in order, off until it is turned on, and each item switched on on its own. */
export type HomeKinds = { login: string; kinds: HomeKindSwitch[]; items: HomeItem[] };

/** `POST /shared_home/repository`'s body: where one profile's home is made, before it exists. `owner` is the profile's login or one of its organisations. Answers `SharedHomeRead`. */
export type SetHomeRepository = { login: string; owner: string; name: string };
