// Where a line of a drawn patch is in the file, and the line of a patch a forge's place names. **The diff draws no
// numbers** (`@armada/screens` `drawn-patch.ts`), so the review surface names a line by its index in the drawn
// lines, and the forge names it by its number in the file on one side. Both meet here, and nowhere else: a comment he
// writes leaves as a place, and a thread the forge holds is found again by its place.

import type { DiffLine } from "@armada/components";
import type { DiffSide } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

/** A line of a file on one side of the change, as the forge numbers it. */
export type Place = { line: number; side: DiffSide };

type Counted = { old: number | undefined; now: number | undefined };

const HEADER = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

/** What each drawn line is numbered, by git's `@@` headers. A header and a line before any header have none. */
function counted(lines: readonly DiffLine[]): Counted[] {
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

/** The lines of one file's patch as the forge printed it (hunks only, no `diff --git` header). */
export function linesOfPatch(path: string, patch: string): DiffLine[] {
  return drawnOf(`diff --git a/${path} b/${path}\n${patch}`, () => "").files[0]?.lines ?? [];
}

/** A patch for lines drawn from one: the inverse of `linesOfPatch`, so a mock forge can serve what a fixture draws. */
export function patchOfLines(lines: readonly DiffLine[]): string {
  return lines.map((one) => one.text).join("\n");
}
