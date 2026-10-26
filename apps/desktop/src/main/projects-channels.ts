// The channels a repository's projects travel over. Each is one request to Fleet; the renderer names a Manifest and no port, and a
// request that names no repository, project or row, or carries no body where one is read, is refused here.

import type { IpcMain } from "electron";

import type { LinkProjectIssue, MoveProjectRow, NameProject, ProjectTask, TickProjectTask } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const body = (value: unknown): value is object => typeof value === "object" && value !== null;

export function handleProjects({ ipc, connection }: Hosts): void {
  const host = () => connection()?.projects;
  ipc.handle(CHANNELS.listProjects, (_event, manifest: unknown) => (named(manifest) ? (host()?.list(manifest) ?? unsent) : unsent));
  ipc.handle(CHANNELS.addProject, (_event, manifest: unknown, one: unknown) => (named(manifest) && body(one) ? (host()?.add(manifest, one as NameProject) ?? unsent) : unsent));
  ipc.handle(CHANNELS.renameProject, (_event, manifest: unknown, project: unknown, one: unknown) =>
    named(manifest) && named(project) && body(one) ? (host()?.rename(manifest, project, one as NameProject) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.deleteProject, (_event, manifest: unknown, project: unknown) =>
    named(manifest) && named(project) ? (host()?.delete(manifest, project) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.addProjectTask, (_event, manifest: unknown, project: unknown, one: unknown) =>
    named(manifest) && named(project) && body(one) ? (host()?.addTask(manifest, project, one as ProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.addProjectSubtask, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.addSubtask(manifest, project, row, one as ProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.editProjectTask, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.editTask(manifest, project, row, one as ProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.linkProjectIssue, (_event, manifest: unknown, project: unknown, one: unknown) =>
    named(manifest) && named(project) && body(one) ? (host()?.linkIssue(manifest, project, one as LinkProjectIssue) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.linkTaskIssue, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.linkTaskIssue(manifest, project, row, one as LinkProjectIssue) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.unlinkTaskIssue, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.unlinkTaskIssue(manifest, project, row, one as LinkProjectIssue) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.removeProjectRow, (_event, manifest: unknown, project: unknown, row: unknown) =>
    named(manifest) && named(project) && named(row) ? (host()?.removeRow(manifest, project, row) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.moveProjectRow, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.moveRow(manifest, project, row, one as MoveProjectRow) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.tickProjectTask, (_event, manifest: unknown, project: unknown, row: unknown, one: unknown) =>
    named(manifest) && named(project) && named(row) && body(one) ? (host()?.tickTask(manifest, project, row, one as TickProjectTask) ?? unsent) : unsent,
  );
}
