import { Box, FilePenLine, Folder, GitBranch, GitCommitHorizontal } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Confirming } from "./TileActs";
import { nameOf } from "./tiles";
import type { ClearCost, TileRow } from "./tiles";

function Line({ Glyph, said, word, figure, children }: { Glyph: LucideIcon; said: string; word: string; figure?: ReactNode; children?: ReactNode }) {
  return (
    <div className="armada-confirm__row">
      <div className="armada-confirm__line">
        <Tooltip label={said}>
          <span className="armada-confirm__mark" role="img" aria-label={said}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-confirm__word">{word}</span>
        {figure}
      </div>
      {children}
    </div>
  );
}

function Mono({ items, label }: { items: readonly string[]; label: string }) {
  return (
    <ul className="armada-confirm__names" aria-label={label}>
      {items.map((one) => (
        <li key={one}>{one}</li>
      ))}
    </ul>
  );
}

const commitsOf = (n: number) => (n === 1 ? "1 commit" : `${n} commits`);

/** The unmerged branch and the tip its commits are reachable from. */
function BranchLines({ branch, kept }: { branch: NonNullable<ClearCost["branch"]>; kept: boolean }) {
  return (
    <Line Glyph={GitBranch} said={kept ? "Branch kept, with commits nothing else has" : "Branch deleted"} word={kept ? "Branch kept" : "Branch deleted"}>
      <Mono label="Branch" items={[`${branch.name} · ${commitsOf(branch.commits)} · ${branch.tip}`]} />
    </Line>
  );
}

/**
 * What an act is about to end, named before it is sent, in the panel it was
 * pressed in. **Clear names the files it destroys, with how long they have
 * sat, and the branch it leaves standing**; a reclaim is never a force, and
 * deleting the branch is its own act with its own confirm at the tip shown.
 * The act is destructive in colour only where something is ended for good.
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
  const ends = which !== "clear" || cost.files.length > 0;
  const verb = which === "clear" ? "Clear" : which === "branch" ? "Delete branch" : "Forget Job";
  return (
    <div className="armada-confirm" role="group" aria-label={`${verb} ${nameOf(row)}`}>
      {which === "clear" ? (
        <>
          <Line Glyph={Folder} said="Checkout removed" word="Checkout">
            <Mono label="Checkout" items={[held.path]} />
          </Line>
          {cost.files.length === 0 ? null : (
            <Line
              Glyph={FilePenLine}
              said="Written and committed nowhere: the checkout is the only copy"
              word="Destroyed"
              figure={
                row.sat === undefined ? null : (
                  <Tooltip label={`Last moved ${row.sat} ago`}>
                    <span className="armada-confirm__figure" aria-label={`Last moved ${row.sat} ago`}>
                      {row.sat}
                    </span>
                  </Tooltip>
                )
              }
            >
              <Mono label="Destroyed files" items={cost.files} />
            </Line>
          )}
          {cost.branch === undefined ? null : <BranchLines branch={cost.branch} kept />}
        </>
      ) : null}
      {which === "branch" && cost.branch !== undefined ? (
        <>
          <BranchLines branch={cost.branch} kept={false} />
          <Line Glyph={GitCommitHorizontal} said="The commits are reachable from this tip alone" word="Tip">
            <Mono label="Tip" items={[cost.branch.tip]} />
          </Line>
        </>
      ) : null}
      {which === "forget" ? (
        <Line Glyph={Box} said="No undo, and the Job cannot be opened again" word="Record forgotten">
          <Mono label="Job" items={[held.job_title]} />
        </Line>
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
