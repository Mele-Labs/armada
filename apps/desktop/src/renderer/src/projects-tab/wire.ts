// What Fleet lists, read into what the tab draws. Fleet's rows are flat, a parent followed by its subtasks; the tab nests them.
// A tracker's reference is `7` on the wire and `#7` where the tab and the Issues tab name it. **A project is his and spans
// repositories**: every row names its own, and the tasks in no project come as `loose`.

import type { ProjectRow as WireRow, Projects, TaskIssue, TaskPull } from "@armada/protocol";

import type { IssueRow, Linked, LinkedPull, Project, ProjectRow, TaskRow } from "./model";

/** An issue as the tracker names it: a bare number is `#7`, any other reference as it is. */
export const referenceOf = (issue: string): string => (/^\d+$/.test(issue.trim()) ? `#${issue.trim()}` : issue.trim());

const linked = (one: TaskIssue): Linked => ({ reference: referenceOf(one.issue), title: one.title ?? "", url: "", state: one.state ?? "open" });
const pulled = (one: TaskPull): LinkedPull => ({ number: one.number, title: one.title ?? "", url: "", state: one.state ?? "open" });

function rowOf(row: WireRow): ProjectRow {
  const manifestId = row.manifest_id;
  if (row.kind === "issue") return { kind: "issue", id: row.id, manifestId, ...linked({ issue: row.issue, ...(row.title === undefined ? {} : { title: row.title }), ...(row.state === undefined ? {} : { state: row.state }) }) } satisfies IssueRow;
  return {
    kind: "task",
    id: row.id,
    manifestId,
    title: row.title,
    ...(row.notes === "" ? {} : { notes: row.notes }),
    done: row.state === "done",
    ...(row.state === "asked" ? { asked: true } : {}),
    ...(row.held_by === undefined ? {} : { job: { id: row.held_by.session_id, title: row.held_by.title ?? "Job" } }),
    ...((row.issues ?? []).length === 0 ? {} : { issues: (row.issues ?? []).map(linked) }),
    ...((row.pulls ?? []).length === 0 ? {} : { pulls: (row.pulls ?? []).map(pulled) }),
  } satisfies TaskRow;
}

/** A project's rows, each task's subtasks under it. */
function rowsOf(rows: readonly WireRow[]): ProjectRow[] {
  const out: ProjectRow[] = [];
  const tops = new Map<string, TaskRow>();
  for (const row of rows) {
    if (row.kind === "task" && row.parent !== undefined) continue;
    const read = rowOf(row);
    out.push(read);
    if (read.kind === "task") tops.set(read.id, read);
  }
  for (const row of rows) {
    if (row.kind !== "task" || row.parent === undefined) continue;
    const parent = tops.get(row.parent);
    if (parent !== undefined) parent.subtasks = [...(parent.subtasks ?? []), rowOf(row) as TaskRow];
  }
  return out;
}

/** His projects, in his order. `used` is when each project was last used on this Mac, by id. */
export function projectsOf(wire: Projects, used: ReadonlyMap<string, number>): Project[] {
  return wire.projects.map((project) => ({ id: project.id, name: project.name, rows: rowsOf(project.rows), usedAt: used.get(project.id) ?? 0 }));
}

/** The tasks in no project, of every repository. */
export function looseOf(wire: Projects): ProjectRow[] {
  return rowsOf(wire.loose);
}
