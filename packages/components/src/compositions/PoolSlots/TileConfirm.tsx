import { Box, Folder, GitBranch, GitBranchMinus } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { ClearSaves } from "./ClearSaves";
import { Line, Mono } from "./ConfirmLine";
import type { Confirming } from "./TileActs";
import { nameOf } from "./tiles";
import type { ClearCost, TileRow } from "./tiles";

const commitsOf = (n: number) => (n === 1 ? "1 commit" : `${n} commits`);

/** What the branch comes to, in git's terms: kept for commits the base lacks, deleted where the base has them all. */
function BranchEffect({ row, cost }: { row: TileRow; cost: ClearCost }) {
  const name = cost.branch?.name ?? row.held?.branch ?? row.slot?.branch ?? "its branch";
  if (cost.branch !== undefined) {
    const { commits, base, tip } = cost.branch;
    return (
      <Line Glyph={GitBranch} said="Reclaiming never deletes commits that only this branch has" word={`Keeps branch ${name}: ${commitsOf(commits)} not on ${base}`}>
        <Mono label="Branch tip" items={[tip]} />
      </Line>
    );
  }
  const unknown = row.held?.held.some((reason) => reason.why === "base_unanswered") === true;
  const base = row.slot?.base ?? "main";
  return (
    <Line
      Glyph={GitBranch}
      said={unknown ? "Git could not name the base, so the branch is kept" : "Every commit on the branch is on the base"}
      word={unknown ? `Keeps branch ${name}: base branch unknown` : `Deletes branch ${name}: merged into ${base}`}
    />
  );
}

/**
 * What an act will do, listed as the git effects it has and nothing else,
 * before it is sent. **Clear with uncommitted files commits them to the
 * branch first** (`ClearSaves`); with none it lists the worktree it removes or
 * the slot it releases, and what becomes of the branch. A bay is released to
 * the pool and its directory stays; a worktree outside the pool is removed.
 * Delete branch and Forget Job are destructive in colour: Clear loses nothing.
 */
export function TileConfirm({
  which,
  row,
  cost,
  onSend,
  onCancel,
}: {
  which: Confirming;
  row: TileRow;
  cost: ClearCost;
  onSend: () => void;
  onCancel: () => void;
}) {
  const held = row.held!;
  const name = nameOf(row);
  const pooled = row.slot !== undefined;
  const ends = which !== "clear";
  const verb = which === "clear" ? "Clear" : which === "branch" ? "Delete branch" : "Forget Job";
  return (
    <div className="armada-confirm" role="group" aria-label={`${verb} ${name}`}>
      {which === "clear" && cost.files.length > 0 ? (
        <ClearSaves branch={held.branch} files={cost.files} path={held.path} {...(pooled ? { slot: name } : {})} />
      ) : null}
      {which === "clear" && cost.files.length === 0 ? (
        <>
          {pooled ? (
            <>
              <Line Glyph={Folder} said="The pool takes the slot back for its next lease" word={`Releases ${name} to the pool`} />
              <Line
                Glyph={Folder}
                said="The directory and its build stay for the next lease"
                word={`Keeps the worktree at .armada/slots/${name}, detached from ${held.branch}`}
              />
            </>
          ) : (
            <Line Glyph={Folder} said="git worktree remove, with its files" word={`Removes the worktree at ${held.path}`} />
          )}
          <BranchEffect row={row} cost={cost} />
        </>
      ) : null}
      {which === "branch" && cost.branch !== undefined ? (
        <>
          <Line Glyph={GitBranchMinus} said="git branch -D" word={`Deletes branch ${cost.branch.name} at ${cost.branch.tip}`} />
          <Line
            Glyph={GitBranch}
            said="Only the tip commit gets them back"
            word={`${commitsOf(cost.branch.commits)} not on ${cost.branch.base} stay reachable only from ${cost.branch.tip}`}
          />
        </>
      ) : null}
      {which === "forget" ? (
        <>
          <Line Glyph={Box} said="There is no undo, and the Job cannot be opened again" word={`Deletes the record of Job ${row.job ?? held.job_title}`} />
          <Line Glyph={Folder} said="The worktree and branch are already gone or kept as they are" word="Leaves the worktree and branch alone" />
        </>
      ) : null}
      <div className="armada-confirm__choice">
        <Button variant={ends ? "destructive" : "secondary"} size="sm" onClick={onSend}>
          {verb}
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
