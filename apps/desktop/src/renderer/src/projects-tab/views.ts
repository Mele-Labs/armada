// What the tab lists: a view (all the tasks, the ones Fleet asks about, the ones a Job holds), a repository (all its tasks), or a
// project (its cards in his order). A view or a repository is cut into groups by project, the tasks in no project first, each under
// a small quiet head; a project is one group with no head.

import { Bell, CircleDot, Layers, type LucideIcon } from "lucide-react";

import type { Project, ProjectRow } from "./model";
import { isDone, rowState, type ProjectsSnapshot } from "./store";

export type View = "all" | "needs" | "held";

export const VIEW_IDS: readonly View[] = ["all", "needs", "held"];

/** Each view's glyph and the word its tooltip and its row name it by. The held glyph is the card's own for a task in a Job. */
export const VIEW_MARKS: Readonly<Record<View, { Glyph: LucideIcon; said: string }>> = {
  all: { Glyph: Layers, said: "All tasks" },
  needs: { Glyph: Bell, said: "Needs you" },
  held: { Glyph: CircleDot, said: "In a Job" },
};

/** The head of the tasks in no project. */
export const NO_PROJECT = "No project";

export type Group = {
  key: string;
  /** The head's words; none for a project's own pane, which is one list. */
  title: string | undefined;
  rows: ProjectRow[];
  /** The cards are moved among each other by a drag and ⌥↑ ⌥↓: only a project's own pane. */
  movable: boolean;
};

const matches = (view: View, row: ProjectRow): boolean => view === "all" || rowState(row) === (view === "needs" ? "asking" : "held");

/** A row drawn: done ones only while they are shown. */
export const shown = (row: ProjectRow, showDone: boolean): boolean => showDone || !isDone(row);

/** A view's or a repository's groups: the tasks in no project first, then each project in the list's order. A group with no card is not drawn. */
export function groupsOf(snap: Pick<ProjectsSnapshot, "projects" | "loose" | "showDone">, view: View, repo: string | undefined): Group[] {
  const keep = (rows: readonly ProjectRow[]) => rows.filter((row) => matches(view, row) && shown(row, snap.showDone) && (repo === undefined || row.manifestId === repo));
  const loose = keep(snap.loose);
  return [
    ...(loose.length === 0 ? [] : [{ key: "none", title: NO_PROJECT, rows: loose, movable: false }]),
    ...snap.projects.flatMap((project) => {
      const rows = keep(project.rows);
      return rows.length === 0 ? [] : [{ key: project.id, title: project.name, rows, movable: false }];
    }),
  ];
}

/** A project's own pane: its cards in his order, none grouped. */
export const groupOf = (project: Project, showDone: boolean): Group[] => {
  const rows = project.rows.filter((row) => shown(row, showDone));
  return rows.length === 0 ? [] : [{ key: project.id, title: undefined, rows, movable: true }];
};

/** The repositories the cards of these groups stand in. */
export const repositoriesIn = (groups: readonly Group[]): string[] => [...new Set(groups.flatMap((group) => group.rows.map((row) => row.manifestId)))];
