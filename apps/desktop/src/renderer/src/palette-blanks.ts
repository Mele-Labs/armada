// The palette's second step for a Commands mod's row that has blanks: one step a blank, in the order they first appear in the text.
// A blank named `pr`, `issue` or `repo` lists what can fill it; any other is a word typed in the field.

import { CircleDot, GitPullRequest } from "lucide-react";
import type { PaletteEntry, PaletteStep } from "@armada/components";
import type { PaletteLinked } from "./palette-fixtures";
import type { Filled } from "./palette-rows";
import type { ModRow } from "./mod-commands";

export type BlankWorld = {
  pulls: readonly PaletteLinked[];
  issues: readonly PaletteLinked[];
  repositories: readonly { root: string; label: string }[];
  /** Every blank has its word: run the row. */
  run: (row: ModRow, filled: Filled) => void;
};

/** `focus_area` as a heading: Focus area. */
const titled = (name: string): string => `${name.charAt(0).toUpperCase()}${name.slice(1).replace(/_/g, " ")}`;

const ticket = (one: PaletteLinked, icon: typeof GitPullRequest, section: string, kind: string): PaletteEntry => ({
  id: `${kind}:${one.repo}:${one.number}`,
  section,
  label: one.title,
  icon,
  meta: { repo: one.repo, number: `#${one.number}` },
});

/** The step that asks for blank `at` of the row, given what the earlier ones were filled with. */
export function blankStep(row: ModRow, at: number, filled: Filled, world: BlankWorld): PaletteStep {
  const name = row.command.blanks![at]!;
  const next = (text: string, bare: string): PaletteStep | void => {
    const held = { ...filled, [name]: { text, bare } };
    if (at + 1 < row.command.blanks!.length) return blankStep(row, at + 1, held, world);
    world.run(row, held);
  };
  if (name === "pr" || name === "issue") {
    const pulls = name === "pr";
    const from = pulls ? world.pulls : world.issues;
    const section = pulls ? "pulls" : "issues";
    return {
      title: pulls ? "Pull request" : "Issue",
      sections: [{ id: section, title: pulls ? "Open pull requests" : "Issues" }],
      entries: from.map((one) => ticket(one, pulls ? GitPullRequest : CircleDot, section, name)),
      searched: pulls ? "every open pull request" : "every open issue",
      onChoose: (entry) => {
        const one = from.find((held) => `${name}:${held.repo}:${held.number}` === entry.id);
        if (one !== undefined) return next(`#${one.number}`, `${one.number}`);
      },
    };
  }
  if (name === "repo") {
    return {
      title: "Repository",
      sections: [{ id: "repos", title: "Repositories" }],
      entries: world.repositories.map((one) => ({ id: `repo:${one.root}`, section: "repos", label: one.label })),
      searched: "every repository",
      onChoose: (entry) => next(entry.label, entry.label),
    };
  }
  return { title: titled(name), sections: [], entries: [], searched: "", word: true, onChoose: (entry) => next(entry.label, entry.label) };
}
