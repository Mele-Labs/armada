// The store over a Fleet that keeps his projects across repositories: a task is written into a repository whatever project it
// joins, the tasks in no project are loose, and what he deletes comes back in the repository it was in.

import { expect, test, vi } from "vitest";
import type { ProjectRow, Projects } from "@armada/protocol";

import type { ProjectsApi } from "../../../shared/api/projects";
import type { TaskRow } from "./model";
import { createProjectsStore, lastUsedRepo } from "./store";

const task = (id: string, manifest_id: string, title: string): ProjectRow => ({ kind: "task", id, manifest_id, title, notes: "", state: "open" });

/** Fleet as far as the store reads it: each write is noted, applied to what Fleet holds, and answered with the projects whole. */
function fleet() {
  const held: Projects = {
    projects: [{ id: "p1", name: "Pocket launch", rows: [task("a1", "armada", "Pair a phone"), task("s1", "site", "Add the badge")] }],
    loose: [task("l1", "site", "Fix the sitemap")],
  };
  let made = 0;
  const answer = async () => ({ ok: true, value: structuredClone(held) }) as const;
  const rowsOf = (project: string | undefined) => (project === undefined ? held.loose : held.projects.find((one) => one.id === project)!.rows);
  return {
    held,
    listProjects: vi.fn(answer),
    addProject: vi.fn(async (body: { name: string }) => {
      held.projects.push({ id: `p${(made += 1) + 1}`, name: body.name, rows: [] });
      return answer();
    }),
    deleteProject: vi.fn(async (project: string) => {
      held.projects = held.projects.filter((one) => one.id !== project);
      return answer();
    }),
    addProjectTask: vi.fn(async (body: { manifest_id: string; project?: string; title: string }) => {
      rowsOf(body.project).push(task(`n${(made += 1)}`, body.manifest_id, body.title));
      return answer();
    }),
    moveProjectRow: vi.fn(async (row: string, body: { position: number }) => {
      for (const rows of [held.loose, ...held.projects.map((one) => one.rows)]) {
        const from = rows.findIndex((one) => one.id === row);
        if (from >= 0) rows.splice(body.position, 0, rows.splice(from, 1)[0]!);
      }
      return answer();
    }),
    removeProjectRow: vi.fn(async (row: string) => {
      held.loose = held.loose.filter((one) => one.id !== row);
      for (const one of held.projects) one.rows = one.rows.filter((it) => it.id !== row);
      return answer();
    }),
    moveProjectTaskRepository: vi.fn(async (row: string, body: { manifest_id: string }) => {
      for (const one of held.projects.flatMap((it) => it.rows)) if (one.kind === "task" && (one.id === row || one.parent === row)) one.manifest_id = body.manifest_id;
      return answer();
    }),
    onProjectsChanged: () => () => {},
  };
}

async function loaded(source: ReturnType<typeof fleet>) {
  const store = createProjectsStore(source as unknown as ProjectsApi);
  store.load();
  await vi.waitFor(() => expect(store.get().projects).toHaveLength(1));
  return store;
}

test("a project holds rows of two repositories in the order Fleet lists them, and the tasks in none are loose", async () => {
  const store = await loaded(fleet());

  expect(store.get().projects[0]!.rows.map((row) => [row.id, row.manifestId])).toEqual([
    ["a1", "armada"],
    ["s1", "site"],
  ]);
  expect(store.get().loose.map((row) => [row.id, row.manifestId])).toEqual([["l1", "site"]]);
});

test("a task added to a project is written into its own repository and put at the top, whatever repository the project's others are in", async () => {
  const source = fleet();
  const store = await loaded(source);

  store.addTask("site", "p1", "Write the pairing page");

  await vi.waitFor(() => expect(source.moveProjectRow).toHaveBeenCalledTimes(1));
  expect(source.addProjectTask).toHaveBeenCalledWith({ manifest_id: "site", project: "p1", title: "Write the pairing page" });
  expect(source.moveProjectRow).toHaveBeenCalledWith("n1", { position: 0 });
  await vi.waitFor(() => expect(store.get().projects[0]!.rows.map((row) => row.title)).toEqual(["Write the pairing page", "Pair a phone", "Add the badge"]));
});

test("a task added to no project is a loose task of its repository and names no project", async () => {
  const source = fleet();
  const store = await loaded(source);

  store.addTask("armada", undefined, "Update the changelog");

  await vi.waitFor(() => expect(source.moveProjectRow).toHaveBeenCalledTimes(1));
  expect(source.addProjectTask).toHaveBeenCalledWith({ manifest_id: "armada", title: "Update the changelog" });
  await vi.waitFor(() => expect(store.get().loose.map((row) => [row.title, row.manifestId])).toEqual([["Update the changelog", "armada"], ["Fix the sitemap", "site"]]));
});

test("a deleted loose task comes back loose in the repository it was in, at its place", async () => {
  const source = fleet();
  const store = await loaded(source);

  store.remove("l1");
  await vi.waitFor(() => expect(source.removeProjectRow).toHaveBeenCalledWith("l1"));
  store.undo();

  await vi.waitFor(() => expect(source.moveProjectRow).toHaveBeenCalledTimes(1));
  expect(source.addProjectTask).toHaveBeenCalledWith(expect.objectContaining({ manifest_id: "site", title: "Fix the sitemap" }));
  expect(source.addProjectTask.mock.calls[0]![0]).not.toHaveProperty("project");
});

test("a deleted project is one write, and coming back it is one new project with each row in its own repository", async () => {
  const source = fleet();
  const store = await loaded(source);

  store.removeProject("p1");
  await vi.waitFor(() => expect(source.deleteProject).toHaveBeenCalledWith("p1"));
  store.undo();

  await vi.waitFor(() => expect(source.addProjectTask).toHaveBeenCalledTimes(2));
  expect(source.addProject).toHaveBeenCalledWith({ name: "Pocket launch" });
  const written = source.addProjectTask.mock.calls.map(([body]) => [body.manifest_id, body.title]);
  expect(written).toEqual([
    ["armada", "Pair a phone"],
    ["site", "Add the badge"],
  ]);
});

test("a row Dispatch starts a Job with is named by its id alone", async () => {
  const store = await loaded(fleet());

  expect(store.refOf("s1")).toEqual({ row: "s1" });
  expect(store.refOf("nope")).toBeUndefined();
});

const shown = (id: string, manifestId: string, subtasks?: TaskRow[]): TaskRow => ({ kind: "task", id, manifestId, title: id, done: false, ...(subtasks === undefined ? {} : { subtasks }) });

test("the repository last used is this window's last add, else the newest task's, else none", () => {
  const rows = [shown("01J0A", "armada"), shown("01J0C", "site"), shown("01J0B", "armada")];
  // Ids are ULIDs, so the greatest is the newest, wherever it stands in the lists.
  expect(lastUsedRepo({ projects: [{ id: "p", name: "P", rows: rows.slice(0, 2), usedAt: 0 }], loose: rows.slice(2), lastRepo: undefined })).toBe("site");
  // A subtask counts as a task: its parent is older but it is the newest written.
  expect(lastUsedRepo({ projects: [], loose: [shown("01J0A", "armada", [shown("01J0Z", "site")])], lastRepo: undefined })).toBe("site");
  expect(lastUsedRepo({ projects: [], loose: rows, lastRepo: "armada" })).toBe("armada");
  expect(lastUsedRepo({ projects: [], loose: [], lastRepo: undefined })).toBeUndefined();
});

test("a task added in this window is the repository last used, before the newest one Fleet lists", async () => {
  const store = await loaded(fleet());
  expect(lastUsedRepo(store.get())).toBe("site");

  store.addTask("armada", undefined, "Update the changelog");

  expect(lastUsedRepo(store.get())).toBe("armada");
});

/** Fleet with a task that has two subtasks, in armada's Pocket launch. */
async function movable() {
  const source = fleet();
  const sub = (id: string, title: string): ProjectRow => ({ kind: "task", id, manifest_id: "armada", title, notes: "", state: "open", parent: "a1" });
  source.held.projects[0]!.rows.splice(1, 0, sub("a1a", "Draw the code"), sub("a1b", "Scan it"));
  return { source, store: await loaded(source) };
}

test("a task moved to another repository takes its subtasks with it, in the same place, and writes one move", async () => {
  const { source, store } = await movable();

  store.moveTaskToRepository("a1", "site");

  const [first, other] = store.get().projects[0]!.rows as [TaskRow, TaskRow];
  expect([first, ...first.subtasks!].map((row) => row.manifestId)).toEqual(["site", "site", "site"]);
  expect([first.id, other.id]).toEqual(["a1", "s1"]);
  await vi.waitFor(() => expect(source.moveProjectTaskRepository).toHaveBeenCalledTimes(1));
  expect(source.moveProjectTaskRepository).toHaveBeenCalledWith("a1", { manifest_id: "site" });
});

test("a subtask is not moved on its own, nor a task to the repository it is in: neither is written", async () => {
  const { source, store } = await movable();

  store.moveTaskToRepository("a1a", "site");
  store.moveTaskToRepository("a1", "armada");

  const parent = store.get().projects[0]!.rows[0] as TaskRow;
  expect([parent, ...parent.subtasks!].map((row) => row.manifestId)).toEqual(["armada", "armada", "armada"]);
  // Writes go one after another, so a real move after them is the last to arrive and anything refused would be before it.
  store.moveTaskToRepository("a1", "site");
  await vi.waitFor(() => expect(source.moveProjectTaskRepository).toHaveBeenCalledWith("a1", { manifest_id: "site" }));
  expect(source.moveProjectTaskRepository).toHaveBeenCalledTimes(1);
});
