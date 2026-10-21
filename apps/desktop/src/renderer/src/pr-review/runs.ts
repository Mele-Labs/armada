// What a run of an agent comes to, mapped into the shapes the review surface draws. **Fleet serves a result as one of six
// shapes** (`review-runs.ts`); this is the one place they meet the explanation's themes, the other explanations' accounts,
// an answer's text and a tool's findings.

import type { ReviewFinding, ReviewMatch, ReviewResult, ReviewRun, WalkPlace } from "@armada/protocol";

import type { Account, AgentRun, Explanation, Finding, PatchRead, ToolDef, ToolMark } from "./model";
import { sliceAround } from "./patch";

/** A run is one per kind and name. The head it ran on does not name it: a newer run replaces the older. */
export const runKey = (kind: ReviewRun["kind"], name: string): string => `${kind}:${name}`;

const MARKS: Readonly<Record<string, ToolMark>> = { risks: "risks", "possible-bugs": "bugs", "api-breaking-changes": "breaks", walk: "walk" };

/** A definition Fleet resolved, as the surface holds it. Armada's four tools have a mark each; any other takes the clipboard. */
export const toolDefOf = (tool: { id: string; kind: ToolDef["kind"]; name: string; prompt: string; source: ToolDef["source"]; replaces?: ToolDef["source"]; hides?: ToolDef["source"] }): ToolDef => ({
  id: tool.id,
  kind: tool.kind,
  name: tool.name,
  source: tool.source,
  mark: MARKS[tool.id] ?? "custom",
  checks: tool.prompt,
  agent: true,
  ...(tool.replaces === undefined ? {} : { replaces: tool.replaces }),
  ...(tool.hides === undefined ? {} : { hides: tool.hides }),
});

/** Where a run stands, for the tile, the strip or the row that shows it. A failed run is Fleet's reason, never a result. */
export function agentRunOf(run: ReviewRun): AgentRun {
  const at = Date.parse(run.finished_at ?? run.started_at);
  if (run.state === "failed") return { state: "failed", said: run.failed ?? "", at: Number.isNaN(at) ? Date.now() : at, head: run.head };
  if (run.state === "done") return { state: "done", at: Number.isNaN(at) ? Date.now() : at, head: run.head };
  const since = Date.parse(run.started_at);
  return { state: "running", since: Number.isNaN(since) ? Date.now() : since };
}

/** What an explanation other than How it works says. Themes and findings are not accounts. */
export function accountOf(result: ReviewResult | undefined): Account | undefined {
  if (result === undefined) return undefined;
  switch (result.shape) {
    case "impact":
      return { shape: "change", rows: result.rows };
    case "steps":
      return {
        shape: "steps",
        steps: result.steps.map((step) => ({
          do: step.action,
          look: step.look,
          ...(step.code === undefined ? {} : { code: step.code }),
          from: step.source.from === "description" ? { from: "description" } : { from: "doc", path: step.source.path, address: step.source.path },
        })),
      };
    case "reason":
      return { shape: "reason", problem: result.problem, cost: result.cost, links: result.links ?? [] };
    case "text":
      return result.body === "" ? undefined : { shape: "text", body: result.body };
    default:
      return undefined;
  }
}

/** How it works: the themes with each change's hunk cut from its file's patch once that patch is read, and none before. */
export function explanationOf(result: ReviewResult | undefined, patches: Readonly<Record<string, PatchRead>>): Explanation {
  if (result?.shape !== "themes") return { summary: "", themes: [] };
  return {
    summary: result.summary,
    themes: result.themes.map((theme, at) => ({
      id: `t${at + 1}`,
      title: theme.title,
      purpose: theme.purpose,
      changes: theme.changes.map((one, to) => {
        const read = patches[one.at.path];
        return { id: `t${at + 1}c${to + 1}`, title: one.title, why: one.why, at: one.at, hunks: read?.state === "read" ? [{ path: one.at.path, lines: sliceAround(read.lines, one.at) }] : [] };
      }),
    })),
  };
}

/** What a tool found, with the ids the run's key and the finding's place in the answer give, and none put away yet. */
export function findingsOf(run: ReviewRun): Finding[] {
  if (run.result?.shape !== "findings") return [];
  const key = runKey(run.kind, run.name);
  return run.result.findings.map((one: ReviewFinding, at) => ({ id: `${key}:${at}`, tool: run.name, at: one.at, severity: one.severity, title: one.title, body: one.body, dismissed: false, sent: false }));
}

/** The answer to a question, as Markdown. An answer of no words is none. */
export const answerOf = (result: ReviewResult | undefined): string | undefined => (result?.shape === "text" && result.body !== "" ? result.body : undefined);

/** A place as `path:12` or `path:12-14`, for a line of words that names where a finding is. */
export const placeSaid = (at: { path: string; start: number; end: number }): string => `${at.path}:${at.end > at.start ? `${at.start}-${at.end}` : at.start}`;

/** What he typed, trimmed and single spaced. */
export const wordsOf = (words: string): string => words.trim().replace(/\s+/g, " ");

/** A search's name at Fleet: his words lower cased and single spaced, which is also the run's name. */
export const searchName = (words: string): string => wordsOf(words).toLowerCase();

/** The list a search is drawn as in the dropdown. The prefix keeps it from sharing an id with one of the four. */
export const SEARCH_PREFIX = "search:";
export const searchId = (name: string): string => `${SEARCH_PREFIX}${name}`;

/** The pull requests a search turned up, each with why. A run that is not a finished search has none. */
export const matchesOf = (run: ReviewRun): readonly ReviewMatch[] => (run.state === "done" && run.result?.shape === "matches" ? run.result.matches : []);

/** The walk a run wrote, in whichever shape: the place it was written against and its steps or requests. */
export type WalkWritten = Extract<ReviewResult, { shape: "web_walk" | "api_walk" }>;

export const walkOf = (run: ReviewRun | undefined): WalkWritten | undefined =>
  run?.state === "done" && (run.result?.shape === "web_walk" || run.result?.shape === "api_walk") ? run.result : undefined;

/** A place in words, for a line and a row: where it is, and what starts it where it is pulled down. */
export function placeWords(place: WalkPlace): string {
  const api = place.api === undefined ? "" : `API at ${place.api}`;
  if (place.how === "deployed") return [place.address === undefined ? "" : `Deployed at ${place.address}`, api].filter((one) => one !== "").join(", ");
  return [`Pull it down, run ${place.command ?? ""}`.trim(), place.address === undefined ? "" : `at ${place.address}`, api].filter((one) => one !== "").join(", ");
}

/** An area as it is shown: a directory, with its slash. */
export const areaShown = (area: string): string => (area === "" || area.endsWith("/") ? area : `${area}/`);
