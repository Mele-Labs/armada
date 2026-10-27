// The store over a Fleet that keeps his projects across repositories: a task is written into a repository whatever project it
// joins, the tasks in no project are loose, and what he deletes comes back in the repository it was in.

import { expect, test, vi } from "vitest";
import type { ProjectRow, Projects } from "@armada/protocol";

import type { ProjectsApi } from "../../../shared/api/projects";
import { createProjectsStore } from "./store";

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
