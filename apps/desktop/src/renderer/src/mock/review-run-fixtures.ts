// What the mock Fleet's agent says, as the wire says it: the fixtures of a mock pull request (`review-seed.ts`) converted into the
// shapes of `ReviewResult`, with each place computed from the lines of the hunk the fixture drew.

import type { ReviewPlace, ReviewResult } from "@armada/protocol";

import type { Account, Change, Hunk } from "../pr-review/model";
import { placeOfLine } from "../pr-review/patch";
import type { FindingSeed, MockPr } from "./review-seed";

/** The lines a hunk covers on the head's side, or on the base's where it only removes. */
function placeOfHunk(hunk: Hunk): ReviewPlace {
  const placed = hunk.lines.flatMap((_, at) => placeOfLine(hunk.lines, at) ?? []);
  const right = placed.filter((one) => one.side === "right").map((one) => one.line);
  const numbers = right.length > 0 ? right : placed.map((one) => one.line);
  return { path: hunk.path, side: right.length > 0 ? "right" : "left", start: Math.min(...numbers, Number.MAX_SAFE_INTEGER), end: Math.max(...numbers, 1) };
}

/** The first added line of a hunk, or its first numbered line, as a place of one line. */
function placeOfFirst(hunk: Hunk): ReviewPlace {
  const added = hunk.lines.findIndex((one) => one.kind === "added");
  const at = hunk.lines.findIndex((_, index) => placeOfLine(hunk.lines, index) !== undefined);
  const place = placeOfLine(hunk.lines, added >= 0 ? added : at);
  return place === undefined ? { path: hunk.path, side: "right", start: 1, end: 1 } : { path: hunk.path, side: place.side, start: place.line, end: place.line };
}

const firstHunk = (change: Change | undefined): Hunk => change?.hunks[0] ?? { path: "", lines: [] };

export function themesOf(pr: MockPr): ReviewResult {
  return {
    shape: "themes",
    summary: pr.explanation.summary,
    themes: pr.explanation.themes.map((theme) => ({
      title: theme.title,
      purpose: theme.purpose,
      changes: theme.changes.map((one) => ({ title: one.title, why: one.why, at: placeOfHunk(firstHunk(one)) })),
    })),
  };
}

export function accountResult(account: Account | undefined): ReviewResult {
  if (account === undefined) return { shape: "text", body: "" };
  switch (account.shape) {
    case "change":
      return { shape: "impact", rows: account.rows };
    case "steps":
      return {
        shape: "steps",
        steps: account.steps.map((step) => ({
          action: step.do,
          look: step.look,
          ...(step.code === undefined ? {} : { code: step.code }),
          source: step.from?.from === "doc" ? { from: "doc", path: step.from.path } : { from: "description" },
        })),
      };
    case "reason":
      return { shape: "reason", problem: account.problem, cost: account.cost, links: account.links };
    case "text":
      return { shape: "text", body: account.body };
  }
}

export function findingsResult(pr: MockPr, seeds: readonly FindingSeed[]): ReviewResult {
  const changes = pr.explanation.themes.flatMap((theme) => theme.changes);
  return {
    shape: "findings",
    findings: seeds.map((one) => ({ title: one.title, body: one.body, severity: one.severity ?? "medium", at: placeOfFirst(firstHunk(changes.find((change) => change.id === one.change))) })),
  };
}

export const answerResult = (pr: MockPr, words: string): ReviewResult => ({ shape: "text", body: pr.answers.find((one) => new RegExp(one.when, "i").test(words))?.body ?? "" });
