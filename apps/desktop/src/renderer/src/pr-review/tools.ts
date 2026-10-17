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
 * The tools and explanations of a repository: what Fleet resolved from the three places, with Comments among the tools. Comments is
 * the forge's conversation and Bridge's alone, so it follows Armada's own tools and no definition replaces it.
 */
export function toolsOf(snapshot: Pick<ReviewSnapshot, "tools">, manifestId: string | undefined): readonly ToolDef[] {
  const resolved = manifestId === undefined ? [] : (snapshot.tools[manifestId] ?? []);
  const after = resolved.findIndex((one) => one.id === "api-breaking-changes") + 1;
  return [...resolved.slice(0, after), COMMENTS_TOOL, ...resolved.slice(after)];
}

export const SOURCE_SAID = { armada: "Armada's tool", kit: "Your Kit's tool, in every repository", repository: "This repository's tool, shared with everyone on it" } as const;

/** The tools, which fill the Flight control. */
export const onlyTools = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "tool");

/** The explanations he can choose under the description. */
export const onlyExplanations = (defs: readonly ToolDef[]): readonly ToolDef[] => defs.filter((one) => one.kind === "explanation");

/** The Walk: its press finds where the change runs before it writes, so it does not start a run on its own. */
export const isWalk = (tool: ToolDef): boolean => tool.mark === "walk";

/** How it works has been read, so a result can sit under its themes. */
export const isExplained = (work: PrWork): boolean => work.explains["how-it-works"]?.state === "done";

/** The one tool Fleet does not serve: the forge's conversation and line comments, with his pending review. */
export const COMMENTS_TOOL: ToolDef = { id: "comments", kind: "tool", name: "Comments", source: "armada", mark: "comments", agent: false, checks: "The forge's conversation and line comments, with his pending review." };
