// The one bar's Task and Project: words typed in the palette, filed in his projects without leaving the page he is on. A
// task asks which project, listed in the palette, last used first; a project goes into the repository picked, and on All
// the palette asks which repository first.

import type { AddKind, PaletteStep } from "@armada/components";
import type { RepositorySummary } from "@armada/protocol";

import { bandsIn } from "./list";
import { useProjectsStore } from "./store";

export type Filing = {
  /** Where the words need a place chosen, the step listing the places; `undefined` where they go at once. */
  step: (words: string, kind: AddKind) => PaletteStep | undefined;
  /** Files words that needed no step: a project in the one repository picked. */
  add: (words: string, kind: AddKind) => void;
};

/** What the one bar files, or `undefined` where the window keeps no projects. */
export function useProjectsFiling(picked: RepositorySummary | null, repositories: readonly RepositorySummary[]): Filing | undefined {
  const store = useProjectsStore();
  if (store === undefined) return undefined;
  const bands = bandsIn(picked, repositories);
  return {
    step: (words, kind) => {
      const title = `Add “${words}” to`;
      if (kind === "project") {
        if (bands.length === 1) return undefined;
        return {
          title,
          sections: [{ id: "repositories", title: "Repositories" }],
          entries: bands.map((band) => ({ id: band.manifestId, section: "repositories", label: band.label })),
          searched: "every repository",
          onChoose: (entry) => void store.addProject(entry.id, words, false),
        };
      }
      // The picked repository's projects, or every repository's under its name; the last used first, in both.
      const projects = store
        .get()
        .projects.filter((one) => bands.some((band) => band.manifestId === one.manifestId))
        .sort((a, b) => b.usedAt - a.usedAt);
      const order = [...new Set(projects.map((one) => one.manifestId))];
      return {
        title,
        sections: order.map((id) => ({ id, title: bands.find((band) => band.manifestId === id)?.label ?? id })),
        entries: projects.map((one) => ({ id: one.id, section: one.manifestId, label: one.name })),
        searched: picked === null ? "every repository's projects" : "this repository's projects",
        onChoose: (entry) => store.addTask(entry.id, words),
      };
    },
    add: (words, kind) => {
      if (kind === "project" && bands.length === 1) void store.addProject(bands[0]!.manifestId, words, false);
    },
  };
}
