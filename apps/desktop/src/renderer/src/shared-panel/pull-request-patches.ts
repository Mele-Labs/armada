// Every changed file's diff for the pull request panel's Files tab, drawn from the one read of its patches.

import type { PullRequestPatches, UnifiedDiffProps } from "@armada/components";
import type { PullRequestFilePatch } from "@armada/protocol";
import { drawnOf } from "@armada/screens/src/drawn-patch";

import type { PatchesRead } from "./reads";

/** What a cut patch says. The forge holds the rest, and that is where it is read. */
const cutSentence = (drawnLines: number, total: number): string =>
  `The first ${drawnLines.toLocaleString()} of ${total.toLocaleString()} lines. Open the pull request in the browser for the rest.`;

const NO_CHANGE = "No lines changed in this file.";

/**
 * One file's diff. A file with no patch says Fleet's sentence. The forge's patch has no
 * `diff --git` header, so one is written for the split to name the file by.
 */
export function fileDiffOf(file: Pick<PullRequestFilePatch, "path" | "patch" | "said">): UnifiedDiffProps {
  if (file.patch === undefined) return { files: [], emptyNote: file.said ?? NO_CHANGE };
  const drawn = drawnOf(`diff --git a/${file.path} b/${file.path}\n${file.patch}`, cutSentence);
  if (drawn.files.length === 0) return { files: [], emptyNote: NO_CHANGE };
  return { files: drawn.files, ...(drawn.cut === undefined ? {} : { cut: drawn.cut }) };
}

/** The read as the panel draws it: out while `read` is absent. */
export function patchesOf(read: PatchesRead | undefined): PullRequestPatches {
  if (read === undefined) return { state: "reading" };
  if (!read.ok) return { state: "refused", said: read.said };
  return { state: "read", files: read.files.map((file) => ({ path: file.path, diff: fileDiffOf(file) })) };
}
