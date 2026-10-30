// The mock's Projects answer a move to another repository as Fleet does: the task and its subtasks go, the links stay where they
// were made, and a subtask cannot go on its own.

import { expect, test } from "vitest";
import type { ProjectRow, Projects } from "@armada/protocol";

import { projects } from "./projects";

const task = (id: string, manifest_id: string, more: Partial<Extract<ProjectRow, { kind: "task" }>> = {}): ProjectRow => ({ kind: "task", id, manifest_id, title: id, notes: "", state: "open", ...more });

const SEED = (): Projects => ({
  projects: [
    {
      id: "p1",
      name: "Pocket launch",
      rows: [
        task("t1", "armada", { issues: [{ issue: "2822" }], pulls: [{ number: 2878, manifest_id: "armada" }] }),
        task("t1a", "armada", { parent: "t1" }),
        task("t1b", "armada", { parent: "t1" }),
        task("t2", "armada"),
        { kind: "issue", id: "i1", manifest_id: "armada", issue: "2900" },
      ],
    },
  ],
  loose: [task("l1", "armada")],
});

const make = () => projects.api({ draft: { projectsTab: SEED() } } as never, {} as never);
const ok = async <T extends { ok: boolean }>(answer: Promise<T>) => {
  const read = await answer;
  expect(read.ok).toBe(true);
  return (read as unknown as { value: Projects }).value;
};
const homes = (list: Projects) => Object.fromEntries([...list.loose, ...list.projects.flatMap((one) => one.rows)].map((row) => [row.id, row.manifest_id]));

test("a task and its subtasks move to another repository, and no other row does", async () => {
  const api = make();

  const moved = await ok(api.moveProjectTaskRepository("t1", { manifest_id: "site" }));

  expect(homes(moved)).toEqual({ t1: "site", t1a: "site", t1b: "site", t2: "armada", i1: "armada", l1: "armada" });
  // Where it stands is not changed: the same project, the same order.
  expect(moved.projects[0]!.rows.map((row) => row.id)).toEqual(["t1", "t1a", "t1b", "t2", "i1"]);
  expect(homes(await ok(api.listProjects()))).toEqual(homes(moved));
});

test("a moved task's links stay in the repository they were linked in", async () => {
  const api = make();

  const moved = await ok(api.moveProjectTaskRepository("t1", { manifest_id: "site" }));

  const [first] = moved.projects[0]!.rows;
  expect(first).toMatchObject({ manifest_id: "site", issues: [{ issue: "2822", manifest_id: "armada" }], pulls: [{ number: 2878, manifest_id: "armada" }] });
});

test("a subtask is refused a move of its own, and nothing moves", async () => {
  const api = make();

  const refused = await api.moveProjectTaskRepository("t1a", { manifest_id: "site" });

  expect(refused).toMatchObject({ ok: false, outcome: { error: { code: "fleet.project_subtask_too_deep" } } });
  expect(homes(await ok(api.listProjects()))).toEqual({ t1: "armada", t1a: "armada", t1b: "armada", t2: "armada", i1: "armada", l1: "armada" });
});

test("a row that is not there is refused", async () => {
  const refused = await make().moveProjectTaskRepository("nope", { manifest_id: "site" });

  expect(refused).toMatchObject({ ok: false, outcome: { error: { code: "fleet.no_such_project_row" } } });
});
