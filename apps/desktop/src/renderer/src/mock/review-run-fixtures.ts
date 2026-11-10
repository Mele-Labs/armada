// What the mock Fleet's agent says, as the wire says it: the fixtures of a mock pull request (`review-seed.ts`) converted into the
// shapes of `ReviewResult`, with each place computed from the lines of the hunk the fixture drew.

import type { ReviewFinding, ReviewPlace, ReviewResult } from "@armada/protocol";

import type { Account, Change, Hunk } from "../pr-review/model";
import { counted, placeOfLine } from "../pr-review/patch";
import type { FindingSeed, MockPr } from "./review-seed";

/** The lines a hunk covers on the head's side, or on the base's where it only removes. */
function placeOfHunk(hunk: Hunk): ReviewPlace {
  const placed = hunk.lines.flatMap((_, at) => placeOfLine(hunk.lines, at) ?? []);
  const right = placed.filter((one) => one.side === "right").map((one) => one.line);
  const numbers = right.length > 0 ? right : placed.map((one) => one.line);
  return {
    path: hunk.path,
    side: right.length > 0 ? "right" : "left",
    start: Math.min(...numbers, Number.MAX_SAFE_INTEGER),
    end: Math.max(...numbers, 1),
  };
}

/** The first added line of a hunk, or its first numbered line, as a place of one line. */
function placeOfFirst(hunk: Hunk): ReviewPlace {
  const added = hunk.lines.findIndex((one) => one.kind === "added");
  const at = hunk.lines.findIndex((_, index) => placeOfLine(hunk.lines, index) !== undefined);
  const place = placeOfLine(hunk.lines, added >= 0 ? added : at);
  return place === undefined
    ? {
        path: hunk.path,
        side: "right",
        start: 1,
        end: 1,
      }
    : {
        path: hunk.path,
        side: place.side,
        start: place.line,
        end: place.line,
      };
}

const firstHunk = (change: Change | undefined): Hunk =>
  change?.hunks[0] ?? {
    path: "",
    lines: [],
  };

export function themesOf(pr: MockPr): ReviewResult {
  return {
    shape: "themes",
    summary: pr.explanation.summary,
    themes: pr.explanation.themes.map((theme) => ({
      title: theme.title,
      purpose: theme.purpose,
      changes: theme.changes.map((one) => ({
        title: one.title,
        why: one.why,
        at: placeOfHunk(firstHunk(one)),
      })),
    })),
  };
}

export function accountResult(account: Account | undefined): ReviewResult {
  if (account === undefined)
    return {
      shape: "text",
      body: "",
    };
  switch (account.shape) {
    case "change":
      return {
        shape: "impact",
        rows: account.rows,
      };
    case "steps":
      return {
        shape: "steps",
        steps: account.steps.map((step) => ({
          action: step.do,
          look: step.look,
          ...(step.code === undefined
            ? {}
            : {
                code: step.code,
              }),
          source:
            step.from?.from === "doc"
              ? {
                  from: "doc",
                  path: step.from.path,
                }
              : {
                  from: "description",
                },
        })),
      };
    case "reason":
      return {
        shape: "reason",
        problem: account.problem,
        cost: account.cost,
        links: account.links,
      };
    case "text":
      return {
        shape: "text",
        body: account.body,
      };
  }
}

/** Where a seed's finding sits: its own place, the lines its words name in the change's first hunk, or that hunk's first added line. */
function placeOfSeed(change: Change | undefined, seed: FindingSeed): ReviewPlace {
  if (seed.place !== undefined) return seed.place;
  const hunk = firstHunk(change);
  if (seed.match === undefined) return placeOfFirst(hunk);
  const lineOf = (words: string) => {
    const place = placeOfLine(
      hunk.lines,
      hunk.lines.findIndex((one) => one.text.includes(words)),
    );
    if (place === undefined) throw new Error(`${seed.change} has no line holding “${words}”`);
    return place;
  };
  const from = lineOf(seed.match);
  const to = seed.through === undefined ? from : lineOf(seed.through);
  return {
    path: hunk.path,
    side: from.side,
    start: from.line,
    end: to.line,
  };
}

export function findingsResult(pr: MockPr, seeds: readonly FindingSeed[]): ReviewResult {
  const changes = pr.explanation.themes.flatMap((theme) => theme.changes);
  return {
    shape: "findings",
    findings: seeds.map((one) => ({
      title: one.title,
      body: one.body,
      severity: one.severity ?? "medium",
      at: placeOfSeed(
        changes.find((change) => change.id === one.change),
        one,
      ),
    })),
  };
}

/** What Tour comes to: a stop for each seed, at the lines its words name. */
export function tourResult(pr: MockPr, seeds: readonly FindingSeed[]): ReviewResult {
  const changes = pr.explanation.themes.flatMap((theme) => theme.changes);
  return {
    shape: "tour",
    stops: seeds.map((one) => ({
      title: one.title,
      at: placeOfSeed(
        changes.find((change) => change.id === one.change),
        one,
      ),
    })),
  };
}

export const answerResult = (pr: MockPr, words: string): ReviewResult => ({
  shape: "text",
  body: pr.answers.find((one) => new RegExp(one.when, "i").test(words))?.body ?? "",
});

/** The lines an invented fix replaces a finding's with, and why, by the finding's title. */
const FIXES: Readonly<
  Record<
    string,
    {
      after: string;
      why: string;
    }
  >
> = {
  "Two refunds can pass the same check": {
    after: [
      "    refund = @payment.with_lock do",
      "      raise Refund::TooLarge unless @payment.refundable?(amount_cents)",
      "",
      '      @payment.refunds.create!(amount_cents: amount_cents, state: "pending")',
      "    end",
    ].join("\n"),
    why: "Checks and creates the refund under the payment's row lock, so two requests cannot both pass the check.",
  },
};

/** The head's lines at a place, as the pull request's patch holds them: the page and the fix read the same text. */
function headLines(pr: MockPr, at: ReviewPlace): string {
  const held = new Map<number, string>();
  for (const hunk of pr.explanation.themes.flatMap((theme) =>
    theme.changes.flatMap((change) => change.hunks),
  )) {
    if (hunk.path !== at.path) continue;
    counted(hunk.lines).forEach((one, index) => {
      if (one.now !== undefined && one.now >= at.start && one.now <= at.end)
        held.set(one.now, hunk.lines[index]!.text.slice(1));
    });
  }
  return [
    ...held.entries(),
  ]
    .sort(([a], [b]) => a - b)
    .map(([, text]) => text)
    .join("\n");
}

/** What a fix comes to: the finding's lines from the patch, and the fixture's lines for them. None where no fixture is written. */
export function fixResult(pr: MockPr, finding: ReviewFinding): ReviewResult | undefined {
  const fix = FIXES[finding.title];
  if (fix === undefined) return undefined;
  return {
    shape: "fix",
    at: finding.at,
    before: headLines(pr, finding.at),
    after: fix.after,
    why: fix.why,
  };
}
