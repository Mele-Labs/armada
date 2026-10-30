// The one bar's Task and Project: words typed in the palette, filed in his projects without leaving the page he is on, on one ↵.
// A task's row carries two chips, so where it goes is read before it is filed: the repository, the last used first, and the
// project, No project unless one is chosen. Tab opens either's list; `@` and a repository's name, or `+` and a project's, typed in
// the words set them as well and leave the title. A project is his, so it carries none. The palette also finds a task or subtask
// by its title, with the project (or No project) and the repository under it, and choosing one opens it on the Projects tab
// (`usePaletteTasks`).

import { useEffect, useState } from "react";
import { Folder, FolderGit2 } from "lucide-react";
import type { AddKind, PaletteChip } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";
import type { PaletteTask } from "@armada/shell";

import { bandsIn } from "./list";
import type { ProjectRow } from "./model";
import { NO_PROJECT } from "./views";
import { lastUsedRepo, useProjects, useProjectsStore } from "./store";

/** The project chip's choice for a task in no project. */
const NONE = "none:";

/** The chips' ids, which `chosen` names them by. */
const REPOSITORY = "repository";
const PROJECT = "project";

export type Filing = {
  /** What a task's row carries: the repository and the project it goes to. */
  chips: readonly PaletteChip[];
  /** Files the words: a task where the chips say, a project. */
  add: (words: string, kind: AddKind, chosen?: Readonly<Record<string, string>>) => void;
};

/** What the one bar files, or `undefined` where the window keeps no projects. The chips start over each time the palette opens. */
export function useProjectsFiling(picked: RepositorySummary | null, repositories: readonly RepositorySummary[], open: boolean): Filing | undefined {
  const store = useProjectsStore();
  const snap = useProjects();
  const [repo, pickRepo] = useState<string | undefined>(undefined);
  const [project, pickProject] = useState(NONE);
  useEffect(() => {
    if (open) return;
    pickRepo(undefined);
    pickProject(NONE);
  }, [open]);
  if (store === undefined) return undefined;
  const bands = bandsIn(picked, repositories);
  const last = bands.find((band) => band.manifestId === lastUsedRepo(snap))?.manifestId;
  const repos = [...bands.filter((band) => band.manifestId === last), ...bands.filter((band) => band.manifestId !== last)];
  const home = bands.some((band) => band.manifestId === repo) ? repo! : (repos[0]?.manifestId ?? "");
  const projects = [...snap.projects].sort((a, b) => b.usedAt - a.usedAt);
  const chips: PaletteChip[] = [
    {
      id: REPOSITORY,
      name: "Repository",
      icon: FolderGit2,
      marker: "@",
      options: repos.map((band) => ({ id: band.manifestId, label: band.label, ...(band.manifestId === last ? { detail: "last used" } : {}) })),
      chosen: home,
      onChoose: pickRepo,
    },
    {
      id: PROJECT,
      name: "Project",
      icon: Folder,
      marker: "+",
      options: [{ id: NONE, label: NO_PROJECT }, ...projects.map((one) => ({ id: one.id, label: one.name }))],
      chosen: projects.some((one) => one.id === project) ? project : NONE,
      onChoose: pickProject,
    },
  ];
  return {
    chips,
    add: (words, kind, chosen) => {
      if (kind === "project") return void store.addProject(words, false);
      const into = chosen?.[REPOSITORY] ?? home;
      const list = chosen?.[PROJECT] ?? project;
      if (into !== "") store.addTask(into, list === NONE ? undefined : list, words);
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
