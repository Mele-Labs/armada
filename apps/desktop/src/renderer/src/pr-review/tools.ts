// Which review tools a repository has. **Three places, merged by name, the most specific winning**, the rule a
// workflow's definitions follow (`docs/concepts/workflow.md`): the set Armada carries, then his Kit's at
// `~/.armada/review-tools/`, then the repository's own at `.armada/review-tools/`. A later place keeps the earlier
// tool's position and says whose it replaced; a name nobody had is added at the end.

import type { AgentFixtures, PrWork, ReviewPr, ReviewSnapshot, ToolDef } from "./model";

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

/** The tools Armada carries: the four tools and the four explanations. Mock-backed until wave 2 runs them. */
export const ARMADA_TOOLS: readonly ToolDef[] = [
  { id: "risks", kind: "tool", name: "Risks", source: "armada", mark: "risks", agent: true, checks: "What could go wrong that the change does not handle: shared state, ordering, a caller that was not updated." },
  { id: "bugs", kind: "tool", name: "Possible bugs", source: "armada", mark: "bugs", agent: true, checks: "Behaviour that differs from what the change says it does: an off by one, a case the code skips, a state it never resets." },
  { id: "api-breaking-changes", kind: "tool", name: "API breaking changes", source: "armada", mark: "breaks", agent: true, checks: "Public shapes others depend on: wire fields, exported types and signatures, files a caller reads." },
  { id: "comments", kind: "tool", name: "Comments", source: "armada", mark: "comments", agent: false, checks: "The forge's conversation and line comments, with his pending review." },
  { id: "how-it-works", kind: "explanation", name: "How it works", source: "armada", mark: "custom", agent: true, checks: "How the code does what it does: its changes in the order they were made, each with why." },
  { id: "for-you", kind: "explanation", name: "User Impact", source: "armada", mark: "custom", agent: true, checks: "How this changes things for someone who uses it: what they see or can do differently, before and after." },
  { id: "how-to-test", kind: "explanation", name: "How to Test", source: "armada", mark: "custom", agent: true, checks: "The steps to try this change, each with what to look for. Follow the author's own testing directions in the description first. Where there are none, or they do not cover a changed area, use the nearest docs up the tree from each changed path: READMEs, CLAUDE.md and AGENTS.md, and the docs they point to. Say where each step came from." },
  { id: "why", kind: "explanation", name: "Why", source: "armada", mark: "custom", agent: true, checks: "The problem, and what it cost, with the issue or incident the pull request names." },
];

/** What the agent half has to go on in a window with no mock forge behind it: Armada's own tools, and nothing found. */
export const NO_FIXTURES: AgentFixtures = { built: false, tools: { armada: ARMADA_TOOLS, kit: [], repository: {} }, results: {}, finds: [], prs: {} };
