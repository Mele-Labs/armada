// Which review tools a repository has. **Three places, merged by name, the most specific winning**, the rule a
// workflow's definitions follow (`docs/concepts/workflow.md`): the set Armada carries, then his Kit's at
// `~/.armada/review-tools/`, then the repository's own at `.armada/review-tools/`. A later place keeps the earlier
// tool's position and says whose it replaced; a name nobody had is added at the end.

import type { PrWork, ReviewPr, ReviewSnapshot, ToolDef } from "./model";

/** A name as the file that holds it is called: lower case, words joined by a dash. */
export const idOfName = (name: string): string =>
  name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

export function toolsOf(snapshot: Pick<ReviewSnapshot, "armada" | "kit" | "repository">, manifestId: string | undefined): readonly ToolDef[] {
  const merged: ToolDef[] = [];
  for (const layer of [snapshot.armada, snapshot.kit, manifestId === undefined ? [] : (snapshot.repository[manifestId] ?? [])]) {
    for (const tool of layer) {
      const at = merged.findIndex((one) => one.id === tool.id);
      if (at < 0) merged.push(tool);
      else merged[at] = { ...tool, replaces: merged[at]!.source };
    }
  }
  return merged;
}

export const SOURCE_SAID = { armada: "Armada's tool", kit: "Your Kit's tool, in every repository", repository: "This repository's tool, shared with everyone on it" } as const;

/** The tools, which fill the Flight control. */
export const onlyTools = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "tool");

/** The explanations he can choose under the description. */
export const onlyExplanations = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "explanation");

/** The two a Session of his already knows, where it wrote the pull request. */
export const bySession = (pr: ReviewPr, def: ToolDef): boolean => pr.explainedBy === "session" && (def.id === "how-it-works" || def.id === "why");

/** How it works has been read, so a result can sit under its themes. */
export const isExplained = (work: PrWork): boolean => work.explains["how-it-works"]?.state === "done";
