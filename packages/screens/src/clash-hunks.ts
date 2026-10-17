// Which hunks of two changes to one file collide: both change lines of the file as it was where the two split. Each side's
// patch is from that split, so their `@@ -start,count` ranges count the same file's lines and can be compared.

import type { DiffFile } from "@armada/components";

/** The lines of the file as it was that a hunk header says it changes, as `[from, to)`. A count of nought is a point. */
function rangeOf(header: string): [number, number] | undefined {
  const found = /^@@ -(\d+)(?:,(\d+))? /.exec(header);
  if (found === null) return undefined;
  const from = Number(found[1]);
  const count = found[2] === undefined ? 1 : Number(found[2]);
  return [from, from + Math.max(count, 1)];
}

const meets = (a: [number, number], b: [number, number]) => a[0] < b[1] && b[0] < a[1];

/** `file` with each hunk that collides with a hunk of `other` marked, its mark saying `said`. */
export function markCollisions(file: DiffFile, other: DiffFile, said: string): DiffFile {
  const theirs = other.lines.flatMap((line) => {
    const range = line.kind === "hunk" ? rangeOf(line.text) : undefined;
    return range === undefined ? [] : [range];
  });
  return {
    ...file,
    lines: file.lines.map((line) => {
      if (line.kind !== "hunk") return line;
      const range = rangeOf(line.text);
      return range !== undefined && theirs.some((one) => meets(range, one)) ? { ...line, collides: said } : line;
    }),
  };
}
