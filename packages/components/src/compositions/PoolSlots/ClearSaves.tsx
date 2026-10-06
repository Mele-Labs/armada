import { FilePenLine, Folder, GitBranch } from "lucide-react";

import { Line, Mono } from "./ConfirmLine";

/**
 * What Clear does to a worktree holding uncommitted files: commits them to the
 * Job's branch as a WIP commit, frees the worktree, and keeps the branch.
 * **Nothing is lost and nothing is pushed**, so there is no figure and no tip:
 * the branch moves when the commit is made.
 *
 * A bay is released to the pool; a worktree outside it is removed, which is
 * safe only because the files are committed first. Shared by the Cleanup grid
 * and the Job's own Clear, so both say the same thing.
 */
export function ClearSaves({
  branch,
  files,
  slot,
  path,
}: {
  branch: string;
  files: readonly string[];
  /** The bay's name, where the worktree is one. Absent is a worktree outside the pool. */
  slot?: string;
  path: string;
}) {
  return (
    <>
      <Line
        Glyph={FilePenLine}
        said="git add --all, then git commit. Ignored files are left out and nothing is pushed"
        word={`Commits the uncommitted files to branch ${branch} as a WIP commit`}
      >
        <Mono label="Uncommitted files" items={files} />
      </Line>
      {slot === undefined ? (
        <Line Glyph={Folder} said="git worktree remove. The files are on the branch first" word={`Removes the worktree at ${path}`} />
      ) : (
        <Line Glyph={Folder} said="The pool takes the slot back for its next lease" word={`Releases ${slot}`} />
      )}
      <Line Glyph={GitBranch} said="The WIP commit stays on the branch" word={`Keeps branch ${branch}`} />
    </>
  );
}
