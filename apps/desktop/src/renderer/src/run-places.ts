// What Fleet's run memory and import routes answer, in the shapes the screens draw: places, rows, folders to import and the folder a new project is made in.
// No React, so a node test reaches it. `docs/practices/protocol.md`, *Onboarding*.

import type { Account, RunsPlace, RunsPlaceId, RunsRow, SyncedFolder } from "@armada/components";
import type { ImportFolder, RememberedRun, RunCloud, RunFactKind, RunMemory, RunMemoryPlace, RunPlace, SetRunMemoryPlace } from "@armada/protocol";
import type { RepositorySummary } from "@armada/protocol";
import type { ClaudeFolder, RepositoryRuns, Session } from "@armada/screens/src/draft/sessions";
import { jobIdOf } from "@armada/shell";

const ID: Record<RunPlace, RunsPlaceId> = { this_mac: "mac", repository: "repository", synced: "synced", cloud: "cloud" };
const WIRE: Record<RunsPlaceId, RunPlace> = { mac: "this_mac", repository: "repository", synced: "synced", cloud: "cloud" };

/** `/Users/name/code/my-app` as a person writes it. */
export const homeRelative = (path: string): string => path.replace(/^\/(?:Users|home)\/[^/]+/, "~");

/** What the store row of the place menu starts with, and what a store place is drawn as. */
const STORE = "Armada's store";
const BUCKET = /^(s3|r2|gs):\/\/\S+$/;

/** The login of the profile a repository is bound to, which is whose store view its cloud place is kept in. */
export const boundProfile = (account: Pick<Account, "profiles"> | undefined, root: string): string | undefined =>
  account?.profiles.find((one) => one.repositories?.some((bound) => bound.root === root))?.login;

/** The place Fleet answered, drawn: a synced folder is named by its label where it is one the list knows, else by where it is; a bucket by its address, the store by the profile it is under. */
export function placeOf(wire: RunMemoryPlace | { place: RunPlace; synced_folder?: string; cloud?: RunCloud }, synced: readonly SyncedFolder[], profile?: string): RunsPlace {
  const id = ID[wire.place];
  if (id === "cloud" && wire.cloud !== undefined) return { id, detail: "bucket" in wire.cloud ? wire.cloud.bucket : profile === undefined ? STORE : `${STORE}, ${profile}` };
  if (id !== "synced" || wire.synced_folder === undefined) return { id };
  return { id, detail: synced.find((one) => one.path === wire.synced_folder)?.label ?? homeRelative(wire.synced_folder) };
}

/** A place chosen, as Fleet is asked for it. A synced folder is the path the list gave it, or the path the folder dialog answered; cloud storage is a bucket address or the store. */
export function wireOfPlace(repository: string, place: RunsPlace, synced: readonly SyncedFolder[]): SetRunMemoryPlace {
  if (place.id === "cloud") {
    const detail = place.detail?.trim();
    if (detail !== undefined && BUCKET.test(detail)) return { repository, place: "cloud", cloud: { bucket: detail } };
    return { repository, place: "cloud", ...(detail === undefined || detail.startsWith(STORE) ? { cloud: { store: {} } } : {}) };
  }
  if (place.id !== "synced") return { repository, place: WIRE[place.id] };
  const folder = synced.find((one) => one.label === place.detail)?.path ?? place.detail;
  return { repository, place: "synced", ...(folder === undefined ? {} : { synced_folder: folder }) };
}

/** What a command was run for, as the layer words it. */
export const KIND_WORDS: Record<RunFactKind, string> = {
  tests: "Tests",
  lint: "Lint",
  build: "Build",
  dev_server: "Dev server",
  typecheck: "Typecheck",
  format: "Format",
};

/** The line a fact is known by, which names it to `dropRunFact`. */
export const factKey = (fact: Pick<RememberedRun, "workspace" | "kind" | "command">): string => `${fact.workspace}\u0000${fact.kind}\u0000${fact.command}`;

/** A fact as the layer draws a row. The root is `.` on the wire and no workspace in the row. */
function rowOf(fact: RememberedRun): RunsRow {
  return {
    id: factKey(fact),
    job: KIND_WORDS[fact.kind],
    ...(fact.workspace === "." ? {} : { workspace: fact.workspace }),
    command: fact.command,
    ...(fact.port === undefined ? {} : { port: fact.port }),
    ...(fact.state === "waiting" ? { waiting: fact.why ?? "Not kept yet" } : {}),
  };
}

/** Who saw the commands run, by the tags Fleet supplied. A tag with no words of its own reads as itself. */
const SOURCES: Readonly<Record<string, string>> = { jobs: "Jobs", claude_code_sessions: "Claude Code sessions" };

function sourceOf(memory: RunMemory): string {
  const tags = [...new Set(memory.workspaces.flatMap((one) => one.runs.map((run) => run.source)))];
  const said = tags.map((tag) => SOURCES[tag] ?? tag.replace(/_/g, " ")).join(", ");
  return said === "" ? "" : `From ${said}`;
}

/** All of how a repository runs, as Fleet answered it. */
export function runsOf(name: string, memory: RunMemory, synced: readonly SyncedFolder[], profile?: string): RepositoryRuns {
  return {
    repository: name,
    source: sourceOf(memory),
    rows: memory.workspaces.flatMap((one) => one.runs.map(rowOf)),
    place: placeOf(memory, synced, profile),
  };
}

/** The fact a row names, for dropping it. */
export function factOf(memory: RunMemory, id: string): RememberedRun | undefined {
  return memory.workspaces.flatMap((one) => one.runs).find((run) => factKey(run) === id);
}

/** A folder an agent CLI kept sessions for, as the start draws it. One that is gone cannot be added. */
export function claudeFolderOf(one: ImportFolder): ClaudeFolder {
  return { path: one.path, lastAt: one.last_session_at, git: one.git.state === "root" || one.git.state === "inside" };
}

/** The folder to serve for an imported one: the repository's root where it sits inside one. */
export const rootOfImport = (one: ImportFolder): string => (one.git.state === "inside" ? one.git.root : one.path);

/** Where a new project is made when no folder says otherwise. `~` is the home the app's main process knows. */
export const FALLBACK_PARENT = "~/code";

/**
 * The folder under home that most of the folders sessions were kept for sit in, newest first on a tie, or `~/code`.
 * A folder directly under home is itself the project and says nothing of where projects are kept.
 */
export function defaultParent(paths: readonly string[]): string {
  const counted = new Map<string, number>();
  for (const path of paths) {
    const at = /^(\/(?:Users|home)\/[^/]+\/[^/]+)\/[^/]+/.exec(path);
    if (at?.[1] !== undefined) counted.set(at[1], (counted.get(at[1]) ?? 0) + 1);
  }
  let best: string | undefined;
  for (const [parent, count] of counted) if (best === undefined || count > (counted.get(best) ?? 0)) best = parent;
  return best ?? FALLBACK_PARENT;
}

/** The root of the repository a Job is of: by the Manifest id it carries, else the one whose folder it works in. */
export function rootOfSession(session: Pick<Session, "manifestId" | "cwd">, repositories: readonly RepositorySummary[]): string | undefined {
  const named = session.manifestId === undefined ? undefined : repositories.find((one) => jobIdOf(one) === session.manifestId);
  if (named !== undefined) return named.root;
  const cwd = session.cwd;
  return cwd === undefined ? undefined : repositories.find((one) => cwd === one.root || cwd.startsWith(`${one.root}/`))?.root;
}

/** Whether the person has sent this Job a message. */
export const sentTo = (session: Pick<Session, "rows">): boolean => session.rows.some((row) => row.kind === "message" && row.from.kind === "you");
