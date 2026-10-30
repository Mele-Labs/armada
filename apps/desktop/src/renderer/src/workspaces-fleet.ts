// What System > Workspaces asks of Fleet (armada-cwj): the Workspaces across the repositories this window serves, the autonomy layers that apply to them, and the saves and deletes a press on the
// page comes to. **Fleet is the authority on every refusal**: what it will not keep comes back in its own words and nothing is written. The pure parts are here so they are tried without a window.
//
// `readWorkspaces` is the one place the Workspaces are read from. Fleet answers `list_repository_workspaces` for one repository at a time, so the window asks for each it serves and keeps each id once.

import type { Autonomy, AutonomyKind, AutonomyLayer, AutonomyLayers, Repository, WorkspaceRow } from "@armada/components";
import type { AutonomyLayer as WireLayer, LifecyclePiece, LifecyclePieceKind, Outcome, PieceScope, RepositorySummary, WorkspaceOfRepositories } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { BridgeApi } from "../../shared/api";
import { missing } from "./lifecycle-editing/fleet-pieces";

/** The calls the page uses. */
export type WorkspacesApi = Pick<BridgeApi, "deleteLifecyclePiece" | "listAutonomyDefaults" | "listRepositoryWorkspaces" | "saveLifecyclePiece">;

/** The repositories with a Manifest, which are the ones a Workspace can hold: the Manifest's id and the repository's name. */
export function servedOf(repositories: readonly RepositorySummary[]): Repository[] {
  const served = repositories.flatMap((one): Repository[] => (one.manifest === undefined ? [] : [{ id: one.manifest.id, name: one.manifest.repository }]));
  return served.filter((one, at) => served.findIndex((each) => each.id === one.id) === at);
}

// ---- Workspaces ----------------------------------------------------------------------------------------------------------

/** Each Workspace once. Where Fleet answered one id more than once (a Workspace a repository committed and Kit's copy of it) the one that runs is kept. */
export function uniqueWorkspaces(answers: readonly (readonly WorkspaceOfRepositories[])[]): WorkspaceOfRepositories[] {
  const kept = new Map<string, WorkspaceOfRepositories>();
  for (const one of answers.flat()) {
    const held = kept.get(one.id);
    if (held === undefined || (held.overridden_by !== undefined && one.overridden_by === undefined)) kept.set(one.id, one);
  }
  return [...kept.values()];
}

export type WorkspacesRead = { ok: true; workspaces: WorkspaceOfRepositories[] } | { ok: false; outcome: Outcome };

/** Every Workspace that holds any repository this window serves. The first refusal is the answer where one is refused. */
export async function readWorkspaces(api: Pick<WorkspacesApi, "listRepositoryWorkspaces">, repositories: readonly Repository[]): Promise<WorkspacesRead> {
  const answers = await Promise.all(repositories.map((one) => api.listRepositoryWorkspaces(one.id)));
  const lists: WorkspaceOfRepositories[][] = [];
  for (const answer of answers) {
    if (!answer.ok) return { ok: false, outcome: answer.outcome };
    lists.push(answer.value.workspaces);
  }
  return { ok: true, workspaces: uniqueWorkspaces(lists) };
}

/** A Workspace as the page lists it: only the repositories this window serves, which are the ones a person can tick. */
export function rowOf(one: WorkspaceOfRepositories, served: readonly Repository[]): WorkspaceRow {
  return { id: one.id, name: one.name, repositories: one.repositories.filter((id) => served.some((each) => each.id === id)), ...(one.home === undefined ? {} : { home: one.home }) };
}

/**
 * The id a new Workspace is kept under: its name in lowercase, each run of anything but letters and digits as one `-`. It is not one already taken, and it is never the id of a repository,
 * which Fleet refuses (a scope names a repository or a Workspace by one id).
 */
export function workspaceIdFrom(name: string, taken: Iterable<string>, repositories: Iterable<string>): string {
  const avoid = new Set([...taken, ...repositories].flatMap((one) => [one, one.toLowerCase()]));
  const base = name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "workspace";
  let id = base;
  for (let count = 2; avoid.has(id); count += 1) id = `${base}-${count}`;
  return id;
}

/** What a saved Workspace lists: the repositories ticked, then those of its members this window does not serve, which the page cannot show and a save must not drop. */
export function membersAfter(held: readonly string[], ticked: readonly string[], served: readonly Repository[]): string[] {
  return [...ticked, ...held.filter((id) => !served.some((one) => one.id === id))];
}

// ---- Autonomy ------------------------------------------------------------------------------------------------------------

const KINDS: readonly AutonomyKind[] = ["tracker_write", "forge_write", "draft", "start_job", "start_watch", "tell_agent", "run_command", "run_agent"];
const LEVELS: readonly Autonomy[] = ["acts", "asks", "off"];

/** One file of defaults, at the level the page keys it under: `everywhere`, `workspace:<id>` or `repository:<manifest id>`. */
export type LayerFile = { id: string; level: string; /** Where Fleet kept it: `kit`, `repository` or `armada`. */ source: string; defaults: AutonomyLayer };

/** The level a file sets, by the scope it is for and how narrowly that names the repository it was read for. */
export const levelOf = (scope: PieceScope, reach: string): string => (scope.kind === "everywhere" ? "everywhere" : `${reach === "workspace" ? "workspace" : "repository"}:${scope.id}`);

/** The scope a level's file is saved under. Absent is everywhere. */
export const scopeOf = (level: string): PieceScope | undefined => (level === "everywhere" ? undefined : { kind: "within", id: level.slice(level.indexOf(":") + 1) });

const TIER: Readonly<Record<string, number>> = { everywhere: 0, workspace: 1, repository: 2 };
const tierOf = (level: string): number => TIER[level.split(":")[0]!] ?? 0;

/** The kinds and values a file set that Bridge draws. A kind or value it does not know is left out of the page, not made up. */
function tidy(defaults: WireLayer["defaults"]): AutonomyLayer {
  const out: Partial<Record<AutonomyKind, Autonomy>> = {};
  for (const one of defaults) {
    const kind = KINDS.find((each) => each === one.kind);
    const level = LEVELS.find((each) => each === one.autonomy);
    if (kind !== undefined && level !== undefined) out[kind] = level;
  }
  return out;
}

/** The files Fleet answered, each once, widest level first. A file another of its id runs instead of is not one. */
export function filesOf(answers: readonly (readonly WireLayer[])[]): LayerFile[] {
  const seen = new Set<string>();
  const files: LayerFile[] = [];
  for (const layer of answers.flat()) {
    if (layer.kept.overridden_by !== undefined) continue;
    const level = levelOf(layer.scope, layer.kept.rank.reach);
    const key = `${level}|${layer.id}`;
    if (seen.has(key)) continue;
    seen.add(key);
    files.push({ id: layer.id, level, source: layer.kept.rank.source, defaults: tidy(layer.defaults) });
  }
  return files.sort((a, b) => tierOf(a.level) - tierOf(b.level));
}

export type AutonomyRead = { ok: true; files: LayerFile[] } | { ok: false; outcome: Outcome };

/** Every layer of defaults that applies to a repository this window serves. */
export async function readAutonomy(api: Pick<WorkspacesApi, "listAutonomyDefaults">, repositories: readonly Repository[]): Promise<AutonomyRead> {
  const answers = await Promise.all(repositories.map((one) => api.listAutonomyDefaults(one.id)));
  const lists: WireLayer[][] = [];
  for (const answer of answers) {
    if (!answer.ok) return { ok: false, outcome: answer.outcome };
    lists.push(answer.value.layers);
  }
  return { ok: true, files: filesOf(lists) };
}

/** What each level sets, laid for the page: the files of one level over each other in the order they came, so the last to name a kind sets it. */
export function layersOf(files: readonly LayerFile[]): AutonomyLayers {
  const layers: Record<string, AutonomyLayer> = {};
  for (const file of files) layers[file.level] = { ...(layers[file.level] ?? {}), ...file.defaults };
  return layers;
}

// ---- Acts ----------------------------------------------------------------------------------------------------------------

/** A save or a delete, in the order Fleet is asked. */
export type Act =
  | { act: "save"; piece: LifecyclePiece; overwrite: boolean }
  | { act: "delete"; kind: LifecyclePieceKind; id: string; scope?: PieceScope };

/** The defaults a file is saved with: every kind it set, as the wire names them. */
const recordOf = (defaults: AutonomyLayer): Record<string, string> => Object.fromEntries(Object.entries(defaults).filter((one): one is [string, Autonomy] => one[1] !== undefined));

const autonomyAct = (file: LayerFile | undefined, level: string, defaults: AutonomyLayer): Act => {
  const scope = scopeOf(level);
  // Kit's own file of this id is replaced. A file the repository committed is not Kit's to replace: Kit gets a copy of its id, which runs instead, so the copy carries every kind the file set.
  return { act: "save", overwrite: file?.source === "kit", piece: { kind: "autonomy", autonomy: { id: file?.id ?? "defaults", ...(scope === undefined ? {} : { scope }), defaults: recordOf(defaults) } } };
};

/**
 * What setting a kind at a level, or resetting it (`undefined`), comes to. **One layer for each level**: a set goes in the file that runs last at that level, or in a file called `defaults` where
 * there is none. A reset takes the kind out of every file at the level that names it, and a file left naming nothing is deleted.
 */
export function autonomyActs(files: readonly LayerFile[], level: string, kind: AutonomyKind, value: Autonomy | undefined): Act[] {
  const here = files.filter((one) => one.level === level);
  if (value !== undefined) {
    const file = here[here.length - 1];
    return [autonomyAct(file, level, { ...(file?.defaults ?? {}), [kind]: value })];
  }
  return here.flatMap((file): Act[] => {
    if (file.defaults[kind] === undefined) return [];
    const { [kind]: _reset, ...rest } = file.defaults;
    if (Object.keys(rest).length > 0) return [autonomyAct(file, level, rest)];
    const scope = scopeOf(level);
    // A file the repository committed holds its last kind until the repository changes it.
    return file.source === "kit" ? [{ act: "delete", kind: "autonomy", id: file.id, ...(scope === undefined ? {} : { scope }) }] : [];
  });
}

/** A Workspace is removed from Kit with the layers Kit holds for it. */
export function removalActs(files: readonly LayerFile[], id: string): Act[] {
  const level = `workspace:${id}`;
  return [{ act: "delete", kind: "workspace", id }, ...files.filter((one) => one.level === level && one.source === "kit").map((one): Act => ({ act: "delete", kind: "autonomy", id: one.id, scope: scopeOf(level)! }))];
}

/** A Workspace as it is saved. `overwrite` where Kit holds it, which is where it is not committed in a repository. */
export function workspaceAct(workspace: { id: string; name: string; repositories: string[] }, overwrite: boolean): Act {
  return { act: "save", overwrite, piece: { kind: "workspace", workspace } };
}

/** What a run of acts came to: done, or the words that stopped it. A Fleet that does not serve the route has no refusal to say, so its words are none. */
export type Done = { ok: true } | { ok: false; said: string };

/** Does the acts in order, stopping at the first Fleet refuses. */
export async function perform(api: Pick<WorkspacesApi, "deleteLifecyclePiece" | "saveLifecyclePiece">, acts: readonly Act[]): Promise<Done> {
  for (const one of acts) {
    const answer = one.act === "save" ? await api.saveLifecyclePiece({ piece: one.piece, ...(one.overwrite ? { overwrite: true } : {}) }) : await api.deleteLifecyclePiece({ kind: one.kind, id: one.id, ...(one.scope === undefined ? {} : { scope: one.scope }) });
    if (!answer.ok) return { ok: false, said: missing(answer.outcome) ? "" : refusalWords(answer.outcome) };
  }
  return { ok: true };
}
