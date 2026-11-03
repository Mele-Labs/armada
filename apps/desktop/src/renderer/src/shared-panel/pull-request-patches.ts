// Every changed file's diff for the pull request panel's Files tab, drawn from the one read of its patches.

import type { DiffFile, PullRequestPatches, UnifiedDiffProps } from "@armada/components";
import type { PullRequestFilePatch, PullReviewThread } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import { indexOfPlace, linesOfPatch, sliceAround } from "../pr-review/patch";

import type { PatchesRead } from "./reads";

/** What a cut patch says. The forge holds the rest, and that is where it is read. */
const cutSentence = (drawnLines: number, total: number): string =>
  `The first ${drawnLines.toLocaleString()} of ${total.toLocaleString()} lines. Open the pull request in the browser for the rest.`;

const NO_CHANGE = "No lines changed in this file.";

/**
 * One file's diff. A file with no patch says Fleet's sentence. The forge's patch has no
 * `diff --git` header, so one is written for the split to name the file by.
 */
export function fileDiffOf(
  file: Pick<PullRequestFilePatch, "path" | "patch" | "said">,
): UnifiedDiffProps {
  if (file.patch === undefined)
    return {
      files: [],
      emptyNote: file.said ?? NO_CHANGE,
    };
  const drawn = drawnOf(`diff --git a/${file.path} b/${file.path}\n${file.patch}`, cutSentence);
  if (drawn.files.length === 0)
    return {
      files: [],
      emptyNote: NO_CHANGE,
    };
  return {
    files: drawn.files,
    ...(drawn.cut === undefined
      ? {}
      : {
          cut: drawn.cut,
        }),
  };
}

/** The read as the panel draws it: out while `read` is absent. `lines` is what each diff takes to draw a line's gutter and what hangs under it. */
export function patchesOf(
  read: PatchesRead | undefined,
  lines?: Pick<UnifiedDiffProps, "onLine" | "under">,
): PullRequestPatches {
  if (read === undefined)
    return {
      state: "reading",
    };
  if (!read.ok)
    return {
      state: "refused",
      said: read.said,
    };
  return {
    state: "read",
    files: read.files.map((file) => ({
      path: file.path,
      diff: withLines(fileDiffOf(file), lines),
    })),
  };
}

/** A file with no lines has nothing to put a note on. */
const withLines = (
  diff: UnifiedDiffProps,
  lines: Pick<UnifiedDiffProps, "onLine" | "under"> | undefined,
): UnifiedDiffProps =>
  lines === undefined || diff.files.length === 0
    ? diff
    : {
        ...diff,
        ...lines,
      };

/**
 * The lines of the diff each thread on a line is about, as the review tool cuts them: the line and a few either side, inside
 * its hunk. A thread on a file, a patch not read, or a patch that no longer holds the line draws none.
 */
export function excerptsOf(
  read: PatchesRead | undefined,
): ((thread: PullReviewThread) => DiffFile | undefined) | undefined {
  if (read === undefined || !read.ok) return undefined;
  const patches = new Map(
    read.files.map((file) => [
      file.path,
      file.patch,
    ]),
  );
  return (thread) => {
    const patch = patches.get(thread.path);
    if (thread.line === undefined || patch === undefined) return undefined;
    const lines = linesOfPatch(thread.path, patch);
    const place = {
      line: thread.line,
      side: thread.side,
    };
    if (indexOfPlace(lines, place) < 0) return undefined;
    return {
      path: thread.path,
      lines: sliceAround(lines, {
        path: thread.path,
        side: thread.side,
        start: thread.line,
        end: thread.line,
      }),
    };
  };
}
