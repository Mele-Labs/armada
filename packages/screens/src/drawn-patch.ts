// A served patch, split into the files and lines git wrote, and cut at a bound.
// The Job review and the Manifest surface's run diff both draw patches, so one
// split serves both and neither imports the other's file.
//
// # No line numbers
//
// git states position in the `@@` header and nowhere else. Numbering each line
// would derive a value here and set it beside machine output, where it would
// read as something the repository said.

import type { DiffFile, DiffLine } from "@armada/components";

/**
 * How many lines of a patch are drawn before it is cut.
 *
 * A ceiling and not a guess at a page: it is what keeps a 14,000-line patch
 * from putting 14,000 nodes in the document, which is the freeze the v1 failure
 * log recorded nine times in different words. It goes when a virtualization
 * approach is chosen.
 */
export const DRAWN_LINES = 2000;

/** What the drawn files are, and what was left off the end of them. */
export type Drawn = { files: DiffFile[]; cut?: string };

/**
 * Any patch Fleet served, split and bounded — with the sentence for a cut
 * patch supplied by the caller.
 *
 * **The sentence is the caller's because where the rest lives is.** A Job's
 * patch names the worktree under *Where the work is*; a run in the main
 * checkout has no worktree to name. The split and the bound are one, so the
 * Manifest surface's diff cannot draw a patch differently from a Job's.
 */
export function drawnOf(patch: string, cutSaid: (drawnLines: number, total: number) => string): Drawn {
  const parsed = split(patch);
  return parsed.cutAt === undefined
    ? { files: parsed.files }
    : { files: parsed.files, cut: cutSaid(parsed.cutAt, parsed.total) };
}

/** One file being built as the patch is walked. */
type Building = { path: string; meta?: string; lines: DiffLine[] };

/** The prefix git writes before every file's block. */
const FILE_HEADER = "diff --git ";

/**
 * The header lines that say something about the file rather than about a line
 * of it. `index` is left out: a pair of abbreviated object ids tells a reader
 * nothing they can act on, and it is the noisiest line in every block.
 */
const META = [
  "new file mode ",
  "deleted file mode ",
  "old mode ",
  "new mode ",
  "rename from ",
  "rename to ",
  "copy from ",
  "copy to ",
  "Binary files ",
];

/**
 * A patch that names no file. Fleet renders the patch with `git diff`, which
 * always writes a header, so this is the shape of a reading that broke rather
 * than a case to expect — and it says so rather than drawing a blank row.
 */
const UNNAMED = "(the patch names no file)";

/**
 * The files, the number of lines drawn, and the number the patch held.
 *
 * **`inHunk` is what makes this correct rather than nearly correct.** A content
 * line reading `+++ something` is an added line whose text happens to start
 * with three plusses, and a parser that matched header prefixes anywhere would
 * open a new file in the middle of one — which is the state a diff viewer is
 * least allowed to be in, since the reader would be deciding on a file
 * attributed to the wrong path.
 */
function split(patch: string): { files: DiffFile[]; cutAt?: number; total: number } {
  const rows = patch.split("\n");
  const files: Building[] = [];
  let open: Building | null = null;
  let inHunk = false;
  let drawnSoFar = 0;
  let total = 0;
  let cut = false;

  for (const row of rows) {
    if (row.startsWith(FILE_HEADER)) {
      open = { path: pathOf(row) ?? UNNAMED, lines: [] };
      files.push(open);
      inHunk = false;
      continue;
    }
    if (!inHunk) {
      // The `+++`/`---` pair names the file more reliably than the header
      // does: git escapes both the same way, and the header carries an `a/ b/`
      // pair that a path containing " b/" makes ambiguous. A deletion's `+++`
      // is `/dev/null`, so only a real path replaces what is held.
      if (row.startsWith("+++ ") || row.startsWith("--- ")) {
        const named = stripped(row.slice(4));
        if (open !== null && named !== null) open.path = named;
        continue;
      }
      if (open !== null && META.some((prefix) => row.startsWith(prefix))) {
        open.meta = open.meta === undefined ? row : `${open.meta} \u00b7 ${row}`;
        continue;
      }
    }
    const line = lineOf(row);
    if (line === undefined) continue;
    if (line.kind === "hunk") inHunk = true;
    total += 1;
    if (drawnSoFar >= DRAWN_LINES) {
      cut = true;
      continue;
    }
    if (open === null) {
      open = { path: UNNAMED, lines: [] };
      files.push(open);
    }
    open.lines.push(line);
    drawnSoFar += 1;
  }

  return {
    // A file whose every line fell past the bound is dropped rather than drawn
    // as an empty block: a header with nothing under it reads as a file that
    // changed in no way, which is a claim the patch does not make.
    files: files.filter((file) => file.lines.length > 0),
    ...(cut ? { cutAt: drawnSoFar } : {}),
    total,
  };
}

/**
 * One line of a hunk, or `undefined` where the row is not one.
 *
 * `\ No newline at end of file` is git's own note about the line above it and
 * stays in the block: dropping it would leave a reader thinking a file ends the
 * way every other one does.
 */
function lineOf(row: string): DiffLine | undefined {
  if (row.startsWith("@@")) return { kind: "hunk", text: row };
  if (row.startsWith("+")) return { kind: "added", text: row };
  if (row.startsWith("-")) return { kind: "removed", text: row };
  if (row.startsWith(" ") || row.startsWith("\\")) return { kind: "context", text: row };
  return undefined;
}

/** The path out of a `diff --git a/x b/x` header, or nothing. */
function pathOf(header: string): string | null {
  const rest = header.slice(FILE_HEADER.length);
  const half = Math.floor(rest.length / 2);
  // `a/x b/x` is the same path twice with one space between, so the midpoint
  // is the space on every path that does not itself contain " b/".
  if (rest[half] === " ") return stripped(rest.slice(half + 1));
  const cut = rest.indexOf(" b/");
  return cut === -1 ? null : stripped(rest.slice(cut + 1));
}

/** `a/`, `b/` and `/dev/null` off a header path. `null` where nothing is left. */
function stripped(value: string): string | null {
  const path = value.trim();
  if (path === "" || path === "/dev/null") return null;
  return path.startsWith("a/") || path.startsWith("b/") ? path.slice(2) : path;
}
