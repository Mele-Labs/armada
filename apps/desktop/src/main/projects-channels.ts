// The channels his projects travel over. Each is one request to Fleet; the renderer names no port, and a
// request that names no project or row, or carries no body where one is read, is refused here.

import type { IpcMain } from "electron";

import type { LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, MoveProjectTaskRepository, NameProject, NewProjectIssue, NewProjectTask, ProjectTask, TickProjectTask } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";
const body = (value: unknown): value is object => typeof value === "object" && value !== null;

export function handleProjects({ ipc, connection }: Hosts): void {
  const host = () => connection()?.projects;
  ipc.handle(CHANNELS.listProjects, () => host()?.list() ?? unsent);
  ipc.handle(CHANNELS.addProject, (_event, one: unknown) => (body(one) ? (host()?.add(one as NameProject) ?? unsent) : unsent));
  ipc.handle(CHANNELS.renameProject, (_event, project: unknown, one: unknown) =>
    named(project) && body(one) ? (host()?.rename(project, one as NameProject) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.deleteProject, (_event, project: unknown) => (named(project) ? (host()?.delete(project) ?? unsent) : unsent));
  ipc.handle(CHANNELS.addProjectTask, (_event, one: unknown) => (body(one) ? (host()?.addTask(one as NewProjectTask) ?? unsent) : unsent));
  ipc.handle(CHANNELS.addProjectSubtask, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.addSubtask(row, one as ProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.editProjectTask, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.editTask(row, one as ProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.linkProjectIssue, (_event, one: unknown) => (body(one) ? (host()?.linkIssue(one as NewProjectIssue) ?? unsent) : unsent));
  ipc.handle(CHANNELS.linkTaskIssue, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.linkTaskIssue(row, one as LinkProjectIssue) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.unlinkTaskIssue, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.unlinkTaskIssue(row, one as LinkProjectIssue) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.removeProjectRow, (_event, row: unknown) => (named(row) ? (host()?.removeRow(row) ?? unsent) : unsent));
  ipc.handle(CHANNELS.moveProjectRow, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.moveRow(row, one as MoveProjectRow) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.moveProjectTaskRepository, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.moveTaskRepository(row, one as MoveProjectTaskRepository) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.tickProjectTask, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.tickTask(row, one as TickProjectTask) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.linkTaskJob, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.linkTaskJob(row, one as LinkTaskJob) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.unlinkTaskJob, (_event, row: unknown) => (named(row) ? (host()?.unlinkTaskJob(row) ?? unsent) : unsent));
  ipc.handle(CHANNELS.linkTaskPull, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.linkTaskPull(row, one as LinkTaskPull) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.unlinkTaskPull, (_event, row: unknown, one: unknown) =>
    named(row) && body(one) ? (host()?.unlinkTaskPull(row, one as LinkTaskPull) ?? unsent) : unsent,
  );
}
