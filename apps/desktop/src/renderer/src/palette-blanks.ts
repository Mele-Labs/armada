// The palette's second step for a Commands mod's row that asks for something: one step a blank, in the order they first appear in
// the text. A blank named `pr`, `issue` or `repo` lists what can fill it; any other is a word typed in the field. A shell line also
// needs a repository to run in, which is asked last where the title row has none picked.

import { CircleDot, GitPullRequest } from "lucide-react";
import type { PaletteEntry, PaletteStep } from "@armada/components";
import type { PaletteLinked } from "./palette-fixtures";
import type { Filled } from "./palette-rows";
import type { ModRow } from "./mod-commands";

export type BlankWorld = {
  pulls: readonly PaletteLinked[];
  issues: readonly PaletteLinked[];
  repositories: readonly { root: string; label: string }[];
  /** The repository the title row has picked, by root. */
  picked: string | null;
  /** Everything is asked: run the row. */
  run: (row: ModRow, filled: Filled) => void;
};

/** What each act says on the step that finishes it. */
const VERB = { prompt: "Start a Job", url: "Open", shell: "Run" } as const;

/** What the row asks for, in order: its blanks, and the repository a shell line runs in where none is picked and no blank names one. */
export function askedOf(row: ModRow, picked: string | null): string[] {
  const blanks = row.command.blanks ?? [];
  return row.command.shell !== undefined && picked === null && !blanks.includes("repo") ? [...blanks, "repo"] : [...blanks];
}

/** `focus_area` as a heading: Focus area. */
const titled = (name: string): string => `${name.charAt(0).toUpperCase()}${name.slice(1).replace(/_/g, " ")}`;

const ticket = (one: PaletteLinked, icon: typeof GitPullRequest, section: string, kind: string): PaletteEntry => ({
  id: `${kind}:${one.repo}:${one.number}`,
  section,
  label: one.title,
  icon,
  meta: { repo: one.repo, number: `#${one.number}` },
});

/** The first step the row asks, or nothing where it asks none. */
export const firstStep = (row: ModRow, world: BlankWorld): PaletteStep | undefined => (askedOf(row, world.picked).length === 0 ? undefined : blankStep(row, 0, {}, world));

/** The step that asks for item `at` of the row, given what the earlier ones were filled with. */
export function blankStep(row: ModRow, at: number, filled: Filled, world: BlankWorld): PaletteStep {
  const asked = askedOf(row, world.picked);
  const name = asked[at]!;
  const last = at + 1 === asked.length;
  const act = last ? (row.command.prompt !== undefined ? VERB.prompt : row.command.url !== undefined ? VERB.url : VERB.shell) : "Next";
  const next = (text: string, bare: string, root?: string): PaletteStep | void => {
    const held = { ...filled, [name]: { text, bare, ...(root === undefined ? {} : { root }) } };
    if (!last) return blankStep(row, at + 1, held, world);
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
      act,
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
      act,
      onChoose: (entry) => next(entry.label, entry.label, entry.id.slice("repo:".length)),
    };
  }
  return { title: titled(name), sections: [], entries: [], searched: "", word: true, act, onChoose: (entry) => next(entry.label, entry.label) };
}
