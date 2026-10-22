// What a Job's Field lens reads off a file's change: the counts, the hunks, the rows of the slab's minimap lit where the changed lines
// sit, the newest hunk and how much of it was already read, and which writing drone the card follows. Read once per answer Fleet
// gives, from `git diff` as Fleet printed it. Pure. `docs/concepts/cockpit.md`, *A Job open*.

import type { FileChangeRead } from "@armada/screens/src/draft/sessions";
import { drawnOf } from "@armada/screens/src/drawn-patch";

/** A line of a hunk: what it did, and its text without the marker. */
export type LensLine = { kind: "added" | "removed" | "context"; text: string };

/** A hunk as git wrote it: its `@@` header, where it starts in the old file, and its lines. */
export type LensHunk = { head: string; old: number; lines: readonly LensLine[] };

/** A row of the minimap: nothing changed there, lines added, lines removed, or both. */
export type Row = 0 | 1 | 2 | 3;

/** One file's change as the lens draws it. `cut` is the line Fleet's cut says, where it sent only the start. */
export type LensDiff = { stamp: string; add: number; del: number; hunks: readonly LensHunk[]; rows: readonly Row[]; cut?: string };

/** How many rows the card's slab draws, sampled over the file as far as the change reaches. A smaller slab folds them in pairs. */
export const ROWS = 32;

const HEAD = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,(\d+))? @@/;

/** The change Fleet answered, read once: counts, hunks and the minimap's rows. An `unchanged` answer has no diff to read. */
export function lensOf(read: Extract<FileChangeRead, { ok: true }>): LensDiff {
  const drawn = drawnOf(read.diff, () => "");
  const hunks: LensHunk[] = [];
  // Each changed line's place in the file as it stands: an added line at its own number, a removed one where it was taken out.
  const marks: { at: number; kind: 1 | 2 }[] = [];
  let reach = 0;
  let line = 0;
  let add = 0;
  let del = 0;
  let total = 0;
  for (const file of drawn.files) {
    total += file.lines.length;
    for (const one of file.lines) {
      if (one.kind === "hunk") {
        const [, old, start, count] = HEAD.exec(one.text) ?? [];
        line = Number(start ?? 1);
        reach = Math.max(reach, line + Number(count ?? 1) - 1);
        hunks.push({ head: one.text, old: Number(old ?? 0), lines: [] });
        continue;
      }
      const hunk = hunks.at(-1);
      if (hunk === undefined) continue;
      (hunk.lines as LensLine[]).push({ kind: one.kind, text: one.text.slice(1) });
      if (one.kind === "added") {
        add += 1;
        marks.push({ at: line, kind: 1 });
        line += 1;
      } else if (one.kind === "removed") {
        del += 1;
        marks.push({ at: line, kind: 2 });
      } else line += 1;
    }
  }
  // The file's length is not on the wire: the rows span it as far as the last hunk reaches, and never fewer lines than rows.
  const span = Math.max(ROWS, reach);
  const rows: Row[] = Array.from({ length: ROWS }, () => 0);
  for (const mark of marks) {
    const k = Math.min(ROWS - 1, Math.max(0, Math.floor(((mark.at - 1) / span) * ROWS)));
    rows[k] = (rows[k]! | mark.kind) as Row;
  }
  return { stamp: read.stamp, add, del, hunks, rows, ...(read.cut === true ? { cut: `First ${total.toLocaleString()} lines` } : {}) };
}

/** The minimap folded to `n` rows: each row lit by what any of the rows it takes in is lit by. */
export function foldRows(rows: readonly Row[], n: number): Row[] {
  if (n >= rows.length) return [...rows];
  const per = rows.length / n;
  return Array.from({ length: n }, (_, k) => {
    let out = 0;
    for (let i = Math.floor(k * per); i < Math.floor((k + 1) * per); i += 1) out |= rows[i] ?? 0;
    return out as Row;
  });
}

const sameLine = (a: LensLine, b: LensLine): boolean => a.kind === b.kind && a.text === b.text;
const sameHunk = (a: LensHunk, b: LensHunk): boolean => a.head === b.head && a.lines.length === b.lines.length && a.lines.every((one, k) => sameLine(one, b.lines[k]!));

/**
 * The hunk the card shows from a new answer, and how many of its lines were already read: the last hunk that was not in the answer
 * before, with the lines it shares at its start with the hunk it grew from. A first answer has nothing new to reveal, so all of its
 * last hunk counts as read. `undefined` where no hunk changed.
 */
export function newestOf(was: LensDiff | undefined, now: LensDiff): { hunk: LensHunk; read: number } | undefined {
  const last = now.hunks.at(-1);
  if (last === undefined) return undefined;
  if (was === undefined) return { hunk: last, read: last.lines.length };
  const changed = now.hunks.filter((one) => !was.hunks.some((it) => sameHunk(one, it)));
  const hunk = changed.at(-1);
  if (hunk === undefined) return undefined;
  const from = was.hunks.find((one) => one.old === hunk.old);
  let read = 0;
  if (from !== undefined) while (read < from.lines.length && read < hunk.lines.length && sameLine(from.lines[read]!, hunk.lines[read]!)) read += 1;
  return { hunk, read };
}

/** A drone of the Job writing a file: its key, the file, and when it started writing. */
export type Writer = { key: string; path: string; since: number };

/** The drone the card follows: the one pinned while it writes, else the one that started writing last. None while the pinned one is not writing. */
export function cardOf(writers: readonly Writer[], pinned: string | undefined): Writer | undefined {
  if (pinned !== undefined) return writers.find((one) => one.key === pinned);
  let out: Writer | undefined;
  for (const one of writers) if (out === undefined || one.since >= out.since) out = one;
  return out;
}

/** The writers now, each keeping the time it started writing from `was`; one that is new starts `now`. A move to another file is no new start. */
export function writersOf(was: ReadonlyMap<string, Writer>, now: readonly { key: string; path: string }[], at: number): Map<string, Writer> {
  return new Map(now.map((one) => [one.key, { key: one.key, path: one.path, since: was.get(one.key)?.since ?? at }] as const));
}

/** The counts as the card and a lifted file write them. */
export const countsOf = (diff: Pick<LensDiff, "add" | "del">): string => `+${diff.add} −${diff.del}`;

/** A file lifted off the floor, as the map draws it: the file, whether it is the card's, its minimap and its counts. */
export type Slab = { path: string; card: boolean; rows: readonly Row[]; counts: string };
