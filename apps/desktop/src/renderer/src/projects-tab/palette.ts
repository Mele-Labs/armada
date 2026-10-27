// The one bar's Task and Project: words typed in the palette, filed in his projects without leaving the page he is on. A task asks
// which project, listed in the palette, last used first, or none; it goes to the repository the last task did, else the first
// listed. A project is his, so it asks for no repository. The palette also finds a task or subtask by its title, with the project
// (or No project) and the repository under it, and choosing one opens it on the Projects tab (`usePaletteTasks`).

import type { AddKind, PaletteStep } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { PaletteTask } from "@armada/shell";

import { bandsIn } from "./list";
import type { ProjectRow } from "./model";
import { NO_PROJECT } from "./views";
import { useProjects, useProjectsStore } from "./store";

/** The palette row for a task in no project. */
const NONE = "none:";

export type Filing = {
  /** Where the words need a place chosen, the step listing the places; `undefined` where they go at once. */
  step: (words: string, kind: AddKind) => PaletteStep | undefined;
  /** Files words that needed no step: a project. */
  add: (words: string, kind: AddKind) => void;
};

/** What the one bar files, or `undefined` where the window keeps no projects. */
export function useProjectsFiling(picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): Filing | undefined {
  const store = useProjectsStore();
  if (store === undefined) return undefined;
  const bands = bandsIn(picked, repositories);
  const home = () => bands.find((band) => band.manifestId === store.get().lastRepo)?.manifestId ?? bands[0]?.manifestId;
  return {
    step: (words, kind) => {
      if (kind === "project") return undefined;
      const projects = [...store.get().projects].sort((a, b) => b.usedAt - a.usedAt);
      return {
        title: `Add “${words}” to`,
        sections: [{ id: "projects", title: "Projects" }],
        entries: [{ id: NONE, section: "projects", label: "No project" }, ...projects.map((one) => ({ id: one.id, section: "projects", label: one.name }))],
        searched: "your projects",
        onChoose: (entry) => {
          const repo = home();
          if (repo !== undefined) store.addTask(repo, entry.id === NONE ? undefined : entry.id, words);
        },
      };
    },
    add: (words, kind) => {
      const repo = home();
      if (kind === "project" && repo !== undefined) void store.addProject(words, repo, false);
    },
  };
}

/** Every task and subtask he has, for the palette to find by title: the project it is in, or No project, then its repository. */
export function usePaletteTasks(repositories: readonly RepositorySummary[]): PaletteTask[] {
  const { loose, projects } = useProjects();
  const repository = new Map(bandsIn(null, repositories).map((band) => [band.manifestId, band.label]));
  const named = (rows: ProjectRow[], project: string): PaletteTask[] =>
    rows.flatMap((row) =>
      row.kind === "task" ? [row, ...(row.subtasks ?? [])].map((one) => ({ id: one.id, label: one.title, detail: `${project} · ${repository.get(one.manifestId) ?? one.manifestId}` })) : [],
    );
  return [...named([...loose], NO_PROJECT), ...projects.flatMap((one) => named(one.rows, one.name))];
}
