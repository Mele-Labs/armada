import { DoorOpen, Flame, FolderX, Ghost, KeyRound, LoaderCircle, Snowflake, SquareDashed } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { WorktreeSlot } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Table, TableBody, TableCell, TableRow } from "../../primitives/Table/Table";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A repository's worktree pool, one row per slot: who holds it, whether its
 * build is warm, and how far behind the base it is. `armada worktree --status`
 * is the same reading. Marks are group `Worktree slot` in
 * `packages/icons/icons.toml`.
 */
export type PoolSlotsProps = {
  rows: readonly PoolSlotRow[];
  /** Open the Job holding a slot. */
  onOpenJob: (jobId: string) => void;
};

export type PoolSlotRow = {
  slot: WorktreeSlot;
  /** How long its holder has had it, formatted by the caller's clock. */
  heldFor?: string;
};

type Mark = { Glyph: LucideIcon; said: string; spins?: boolean };

function stateMark(slot: WorktreeSlot): Mark {
  const held = slot.held;
  switch (held.state) {
    case "unmade":
      return { Glyph: SquareDashed, said: "Not made yet" };
    case "not_a_checkout":
      return { Glyph: FolderX, said: "Not a checkout" };
    case "busy":
      return { Glyph: LoaderCircle, said: "Being taken or given back", spins: true };
    case "free":
      return { Glyph: DoorOpen, said: "Free" };
    case "job":
    case "session":
      return { Glyph: KeyRound, said: "Held" };
    case "stranded":
      return { Glyph: Ghost, said: `Stranded: ${held.why}` };
  }
}

function MarkCell({ mark }: { mark: Mark | null }) {
  if (mark === null) return <TableCell />;
  return (
    <TableCell className="armada-pool-slots__mark">
      <Tooltip label={mark.said}>
        <span role="img" aria-label={mark.said} data-spins={mark.spins === true ? "" : undefined}>
          <mark.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </TableCell>
  );
}

function FigureCell({ value, named }: { value: string | undefined; named: string }) {
  if (value === undefined) return <TableCell />;
  return (
    <TableCell variant="mono">
      <Tooltip label={named}>
        <span aria-label={`${named}: ${value}`}>{value}</span>
      </Tooltip>
    </TableCell>
  );
}

function Holder({ slot, onOpenJob }: { slot: WorktreeSlot; onOpenJob: (jobId: string) => void }) {
  const held = slot.held;
  if (held.state === "job") {
    return (
      <Button variant="ghost" size="sm" onClick={() => onOpenJob(held.job_id)}>
        {held.job_title ?? held.job_id}
      </Button>
    );
  }
  if (held.state === "session") return <>{held.holder}</>;
  return null;
}

export function PoolSlots({ rows, onOpenJob }: PoolSlotsProps) {
  return (
    <Table className="armada-pool-slots">
      <TableBody>
        {rows.map(({ slot, heldFor }) => {
          const made = slot.held.state !== "unmade" && slot.held.state !== "not_a_checkout";
          const warmth: Mark | null = !made
            ? null
            : slot.warm
              ? { Glyph: Flame, said: "Warm" }
              : { Glyph: Snowflake, said: "Cold" };
          return (
            <TableRow key={`${slot.manifest_id}/${slot.slot}`}>
              <TableCell variant="mono">
                <Tooltip label={slot.path}>
                  <span>{`slot-${slot.slot}`}</span>
                </Tooltip>
              </TableCell>
              <MarkCell mark={stateMark(slot)} />
              <MarkCell mark={warmth} />
              <TableCell variant="mono">{slot.branch}</TableCell>
              <TableCell>
                <Holder slot={slot} onOpenJob={onOpenJob} />
              </TableCell>
              <FigureCell value={heldFor} named="Held for" />
              <FigureCell
                value={slot.behind === undefined ? undefined : String(slot.behind)}
                named={`Commits behind ${slot.base}`}
              />
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
