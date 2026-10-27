// What the two layout takes list: a view of the tasks across repositories (all, the ones Fleet asks about, the ones a Job
// holds) cut into groups, each with a head. A group is a project, a repository's rows together, or a state.

import { Bell, CircleDot, Layers, type LucideIcon } from "lucide-react";

import type { Band } from "./list";
import type { Project, ProjectRow } from "./model";
import { rowState } from "./store";

export type View = "all" | "needs" | "held";
export type GroupBy = "project" | "repository" | "state";

export const VIEW_IDS: readonly View[] = ["all", "needs", "held"];

/** Each view's glyph and the word its tooltip and its row name it by. The held glyph is the card's own for a task in a Job. */
export const VIEW_MARKS: Readonly<Record<View, { Glyph: LucideIcon; said: string; short: string }>> = {
  all: { Glyph: Layers, said: "All tasks", short: "All" },
  needs: { Glyph: Bell, said: "Needs you", short: "Needs you" },
  held: { Glyph: CircleDot, said: "In a Job", short: "In a Job" },
};
export const GROUP_IDS: readonly GroupBy[] = ["project", "repository", "state"];

/** A row with the project it stands in, which a card needs to dispatch, link and add subtasks. */
export type Entry = { project: Project; row: ProjectRow };

export type Group = {
  key: string;
  /** Where the head names it: a project by repository and name, a state by its word. */
  title: string;
  /** The project the head renames and deletes: a project group's own. */
  project: Project | undefined;
  /** The project a task written at the group's foot joins. */
  adds: Project | undefined;
  /** What a card names of where it stands, where the group mixes projects: nothing, its project, or its repository and project. */
  labelled: "none" | "project" | "repository";
  entries: Entry[];
};

type Options = {
  view: View;
  by: GroupBy;
  /** Only this repository's rows, or every one. */
  repo: string | undefined;
  /** The group heads carry no `repository ·` when one repository is all there is. */
  prefix: boolean;
  /** A project with no rows still has a group, to write the first task in. */
  keepEmpty: boolean;
};

const matches = (view: View, row: ProjectRow): boolean => view === "all" || rowState(row) === (view === "needs" ? "asking" : "held");

const STATES = [
  { id: "asking", title: "Needs you" },
  { id: "held", title: "In a Job" },
  { id: "open", title: "Open" },
  { id: "done", title: "Done" },
] as const;

export function groupsOf(projects: readonly Project[], bands: readonly Band[], { view, by, repo, prefix, keepEmpty }: Options): Group[] {
  const shown = bands.filter((band) => repo === undefined || band.manifestId === repo);
  const under = (band: Band) => {
    const mine = projects.filter((one) => one.manifestId === band.manifestId);
    return [...mine.filter((one) => one.own === true), ...mine.filter((one) => one.own !== true)];
  };
  const entries = (project: Project): Entry[] => project.rows.filter((row) => matches(view, row)).map((row) => ({ project, row }));
  const lead = (band: Band) => (prefix ? `${band.label} · ` : "");

  if (by === "project") {
    return shown.flatMap((band) =>
      under(band).flatMap((project) => {
        const mine = entries(project);
        if (mine.length === 0 && !keepEmpty) return [];
        return [{ key: project.id, title: `${lead(band)}${project.own === true ? "Tasks" : project.name}`, project: project.own === true ? undefined : project, adds: project, labelled: "none", entries: mine }];
      }),
    );
  }
  if (by === "repository") {
    return shown.flatMap((band) => {
      const mine = under(band).flatMap(entries);
      const own = under(band).find((one) => one.own === true);
      if (mine.length === 0 && !keepEmpty) return [];
      return [{ key: band.manifestId, title: band.label, project: undefined, adds: own, labelled: "project", entries: mine }];
    });
  }
  const all = shown.flatMap((band) => under(band).flatMap(entries));
  return STATES.flatMap(({ id, title }) => {
    const mine = all.filter((one) => {
      const state = rowState(one.row);
      return id === "done" ? state === "done" || state === "closed" : state === id;
    });
    return mine.length === 0 ? [] : [{ key: id, title, project: undefined, adds: undefined, labelled: "repository", entries: mine }];
  });
}

/** The label a card carries where its group mixes projects: the repository and the project, or the repository's own tasks. */
export function fromOf(entry: Entry, bands: readonly Band[], labelled: Group["labelled"]): string | undefined {
  if (labelled === "none") return undefined;
  const band = bands.find((one) => one.manifestId === entry.project.manifestId);
  const where = entry.project.own === true ? "Tasks" : entry.project.name;
  return labelled === "repository" && band !== undefined ? `${band.label} · ${where}` : where;
}
