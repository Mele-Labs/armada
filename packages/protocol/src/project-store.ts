// A project's claims store on the wire: what the store setting reads, joining a store by its address or
// none, and deploying one to his own Cloudflare account. `docs/contracts/central-store.md`, A store per
// project and Deploying a store. Mirrors `crates/ipc/src/project_store.rs`.

import type { ProjectStore } from "./projects";

/** `GET /projects/project/:project/store`: everything the store setting of one project shows. */
export type ProjectStoreState = {
  project: string;
  /** The store its file names. Absent is none: its Jobs send nothing. */
  store?: ProjectStore;
  /** The names of the Macs signed in to it, oldest first, as the store answered. Empty with no store. */
  macs: string[];
  /** Why the Macs could not be read, where they could not. */
  macs_unread?: string;
  /** Whether this Mac has joined it, as the profile its repositories are bound to. */
  joined_here: boolean;
  /** Whether Deploy can run now. */
  deploy: StoreDeploy;
  /** Whether joining a store can run now. */
  join: StoreJoin;
  /** Cloudflare's page that makes a token with the rights a deploy needs, filled in. Opened for the second sign-in. */
  token_page: string;
};

/** `ready`: a pasted token is kept. `try_tunnel`: the tunnel's sign-in is tried first and may be refused. `needs_token`: this Mac has no Cloudflare sign-in. */
export type StoreDeploy = "ready" | "try_tunnel" | "needs_token";

/** `ready`: a repository of the project is bound to a profile here. `no_profile`: none is, so there is no one to join as. */
export type StoreJoin = "ready" | "no_profile";

/** `POST /projects/project/:project/store`: the store a project's claims go to, joined by its address, or none. */
export type SetProjectStore = {
  /** Absent is none: its Jobs send nothing from now on. */
  store?: JoinStore;
};

export type JoinStore = {
  address: string;
  /** `org` or `org/team`, for a team's store. */
  team?: string;
};

/** `POST /projects/project/:project/store/deploy`: deploy the project's store to his own Cloudflare account, or redeploy it. */
export type DeployProjectStore = {
  /** `org` or `org/team` to deploy a team's store. Absent is a store of his own. */
  team?: string;
  /** The token he pasted for the second sign-in. Kept in the Keychain once a deploy with it works, and never answered back. */
  cloudflare_token?: string;
};

/** What a deploy did. `needs_token`: Cloudflare refused every sign-in this Mac has, and nothing was changed. */
export type StoreDeployed =
  | { outcome: "deployed"; state: ProjectStoreState; join_said?: string }
  | { outcome: "needs_token"; page: string; said: string };
