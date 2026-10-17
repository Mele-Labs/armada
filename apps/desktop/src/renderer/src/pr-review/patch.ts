// Where a line of a drawn patch is in the file, and the line of a patch a forge's place names. **The diff draws no
// numbers** (`@armada/screens` `drawn-patch.ts`), so the review surface names a line by its index in the drawn
// lines, and the forge names it by its number in the file on one side. Both meet here, and nowhere else: a comment he
// writes leaves as a place, and a thread the forge holds is found again by its place.

import type { DiffLine } from "@armada/components";
import type { DiffSide, ReviewPlace } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

/** A line of a file on one side of the change, as the forge numbers it. */
export type Place = { line: number; side: DiffSide };

export type Counted = { old: number | undefined; now: number | undefined };

const HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** What each drawn line is numbered, by git's `@@` headers. A header and a line before any header have none. */
export function counted(lines: readonly DiffLine[]): Counted[] {
  let old: number | undefined;
  let now: number | undefined;
  return lines.map((one): Counted => {
    if (one.kind === "hunk") {
      const header = HEADER.exec(one.text);
      old = header === null ? undefined : Number(header[1]);
      now = header === null ? undefined : Number(header[2]);
      return { old: undefined, now: undefined };
    }
    if (old === undefined || now === undefined) return { old: undefined, now: undefined };
    if (one.kind === "removed") return { old: old++, now: undefined };
    if (one.kind === "added") return { old: undefined, now: now++ };
    return { old: old++, now: now++ };
  });
}

/** The place of the line at `index`. A removed line is on the left; anything else the head has is on the right. */
export function placeOfLine(lines: readonly DiffLine[], index: number): Place | undefined {
  const at = counted(lines)[index];
  if (at === undefined) return undefined;
  if (at.now !== undefined) return { line: at.now, side: "right" };
  return at.old === undefined ? undefined : { line: at.old, side: "left" };
}

/** The index of the line a place names, or -1 where the patch does not hold it. A context line answers to either side. */
export function indexOfPlace(lines: readonly DiffLine[], place: Place): number {
  return counted(lines).findIndex((one) => (place.side === "right" ? one.now : one.old) === place.line);
}

/** How many lines of the change are kept either side of the place an explanation names. */
export const CONTEXT = 3;

/**
 * The lines of a file's patch around a place, as one hunk: the lines the place covers on its side and `CONTEXT` either side, kept
 * inside the hunks they sit in, under a header numbered where the cut begins so `placeOfLine` still names the forge's lines. A patch
 * that does not hold the place is returned whole.
 */
export function sliceAround(lines: readonly DiffLine[], place: ReviewPlace): DiffLine[] {
  const nums = counted(lines);
  const side = place.side === "right" ? "now" : "old";
  const covered = (at: number) => {
    const number = nums[at]![side];
    return number !== undefined && number >= place.start && number <= place.end;
  };
  const first = nums.findIndex((_, at) => covered(at));
  if (first < 0) return [...lines];
  let last = first;
  for (let at = first; at < lines.length; at += 1) if (covered(at)) last = at;
  let low = first;
  while (low > 0 && lines[low - 1]!.kind !== "hunk") low -= 1;
  let high = last;
  while (high < lines.length - 1 && lines[high + 1]!.kind !== "hunk") high += 1;
  const from = Math.max(low, first - CONTEXT);
  const to = Math.min(high, last + CONTEXT);
  const kept = lines.slice(from, to + 1);
  const header = low > 0 ? HEADER.exec(lines[low - 1]!.text) : null;
  const start = (key: "old" | "now", fallback: number): number => {
    for (let at = from; at <= to; at += 1) {
      if (lines[at]!.kind === "hunk") continue;
      const number = nums[at]![key];
      if (number !== undefined) return number;
    }
    for (let at = from - 1; at >= low; at -= 1) {
      const number = nums[at]![key];
      if (number !== undefined) return number + 1;
    }
    return fallback;
  };
  const inside = nums.slice(from, to + 1);
  const old = start("old", header === null ? 1 : Number(header[1]));
  const now = start("now", header === null ? 1 : Number(header[2]));
  const text = `@@ -${old},${inside.filter((one) => one.old !== undefined).length} +${now},${inside.filter((one) => one.now !== undefined).length} @@`;
  return [{ kind: "hunk", text }, ...kept];
}

/** The lines of one file's patch as the forge printed it (hunks only, no `diff --git` header). */
export function linesOfPatch(path: string, patch: string): DiffLine[] {
  return drawnOf(`diff --git a/${path} b/${path}\n${patch}`, () => "").files[0]?.lines ?? [];
}

/** A patch for lines drawn from one: the inverse of `linesOfPatch`, so a mock forge can serve what a fixture draws. */
export function patchOfLines(lines: readonly DiffLine[]): string {
  return lines.map((one) => one.text).join("\n");
}
