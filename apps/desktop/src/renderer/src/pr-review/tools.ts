// Which review tools a repository has. **Three places, merged by id, the most specific winning**, the rule a
// workflow's definitions follow (`docs/concepts/workflow.md`): the set Armada carries, then his Kit's at
// `~/.armada/review-tools/`, then the repository's own at `.armada/review-tools/`. Fleet does the merge
// (`list_review_tools`); a later place keeps the earlier tool's position and says whose it replaced.

import type { PrWork, ReviewSnapshot, ToolDef } from "./model";

/** A name as the file that holds it is called: lower case, words joined by a dash. */
export const idOfName = (name: string): string =>
  name
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

/**
 * The tools and explanations of a repository: what Fleet resolved from the three places. His review and the forge's comments are
 * not among them: they are the reader's Review tab.
 */
export function toolsOf(snapshot: Pick<ReviewSnapshot, "tools">, manifestId: string | undefined): readonly ToolDef[] {
  return manifestId === undefined ? [] : (snapshot.tools[manifestId] ?? []);
}

export const SOURCE_SAID = { armada: "Armada's tool", kit: "Your Kit's tool, in every repository", repository: "This repository's tool, shared with everyone on it" } as const;

/** The explanations first and then the tools: everything that runs on a pull request, as Flight control's tiles. */
export const tilesOf = (defs: readonly ToolDef[]): readonly ToolDef[] => [...onlyExplanations(defs), ...onlyTools(defs)];

/** The tools. */
export const onlyTools = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "tool");

/** The explanations, each shown under the description when its tile is pressed. */
export const onlyExplanations = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "explanation");

/** The Walk: its press finds where the change runs before it writes, so it does not start a run on its own. */
export const isWalk = (tool: ToolDef): boolean => tool.mark === "walk";

/** How it works has been read, so a result can sit under its themes. */
export const isExplained = (work: PrWork): boolean => work.explains["how-it-works"]?.state === "done";

