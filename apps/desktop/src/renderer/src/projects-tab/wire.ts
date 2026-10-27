// What Fleet lists, read into what the tab draws. Fleet's rows are flat, a parent followed by its subtasks; the tab nests them.
// A tracker's reference is `7` on the wire and `#7` where the tab and the Issues tab name it.

import type { ProjectRow as WireRow, Projects, TaskIssue, TaskPull } from "@armada/protocol";

import type { IssueRow, Linked, LinkedPull, Project, ProjectRow, TaskRow } from "./model";

/** An issue as the tracker names it: a bare number is `#7`, any other reference as it is. */
export const referenceOf = (issue: string): string => (/^\d+$/.test(issue.trim()) ? `#${issue.trim()}` : issue.trim());

const linked = (one: TaskIssue): Linked => ({ reference: referenceOf(one.issue), title: one.title ?? "", url: "", state: one.state ?? "open" });
const pulled = (one: TaskPull): LinkedPull => ({ number: one.number, title: one.title ?? "", url: "", state: one.state ?? "open" });

function rowOf(row: WireRow): ProjectRow {
  if (row.kind === "issue") return { kind: "issue", id: row.id, ...linked({ issue: row.issue, ...(row.title === undefined ? {} : { title: row.title }), ...(row.state === undefined ? {} : { state: row.state }) }) } satisfies IssueRow;
  return {
    kind: "task",
    id: row.id,
    title: row.title,
    ...(row.notes === "" ? {} : { notes: row.notes }),
    done: row.state === "done",
    ...(row.state === "asked" ? { asked: true } : {}),
    ...(row.held_by === undefined ? {} : { job: { id: row.held_by.session_id, title: row.held_by.title ?? "Job" } }),
    ...((row.issues ?? []).length === 0 ? {} : { issues: (row.issues ?? []).map(linked) }),
    ...((row.pulls ?? []).length === 0 ? {} : { pulls: (row.pulls ?? []).map(pulled) }),
  } satisfies TaskRow;
}

/** One repository's projects, each task's subtasks under it. `used` is when each project was last used on this Mac, by id. */
export function projectsOf(wire: Projects, used: ReadonlyMap<string, number>): Project[] {
  return wire.projects.map((project) => {
    const rows: ProjectRow[] = [];
    const tops = new Map<string, TaskRow>();
    for (const row of project.rows) {
      if (row.kind === "task" && row.parent !== undefined) continue;
      const read = rowOf(row);
      rows.push(read);
      if (read.kind === "task") tops.set(read.id, read);
    }
    for (const row of project.rows) {
      if (row.kind !== "task" || row.parent === undefined) continue;
      const parent = tops.get(row.parent);
      if (parent !== undefined) parent.subtasks = [...(parent.subtasks ?? []), rowOf(row) as TaskRow];
    }
    return { id: project.id, manifestId: wire.manifest_id, name: project.name, rows, usedAt: used.get(project.id) ?? 0 };
  });
}
