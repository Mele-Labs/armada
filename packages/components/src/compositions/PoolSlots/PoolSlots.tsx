import { Box, DoorOpen, Flame, FolderX, Ghost, KeyRound, LoaderCircle, Snowflake, SquareDashed } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { WorktreeSlot } from "@armada/protocol";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeaderCell,
  TableRow,
} from "../../primitives/Table/Table";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A repository's worktree pool, one row per slot: who holds it, whether its
 * build is warm, and how far behind the base it is. `armada worktree --status`
 * is the same reading. Marks are group `Worktree slot` in
 * `packages/icons/icons.toml`, in a status hue: leased is in flight, free is
 * ready, stranded needs a person, and a slot not made is neutral.
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

/** Which status hue a mark, and its row, takes. `none` stays neutral. */
type Tone = "leased" | "free" | "stranded" | "warm" | "cold" | "none";

type Mark = { Glyph: LucideIcon; said: string; tone: Tone; spins?: boolean };

function stateMark(slot: WorktreeSlot): Mark {
  const held = slot.held;
  switch (held.state) {
    case "unmade":
      return { Glyph: SquareDashed, said: "Not made yet", tone: "none" };
    case "not_a_checkout":
      return { Glyph: FolderX, said: "Not a checkout", tone: "none" };
    case "busy":
      return { Glyph: LoaderCircle, said: "Being taken or given back", tone: "leased", spins: true };
    case "free":
      return { Glyph: DoorOpen, said: "Free", tone: "free" };
    case "job":
    case "session":
      return { Glyph: KeyRound, said: "Held", tone: "leased" };
    case "stranded":
      return { Glyph: Ghost, said: `Stranded: ${held.why}`, tone: "stranded" };
  }
}

function MarkCell({ mark }: { mark: Mark | null }) {
  if (mark === null) return <TableCell />;
  return (
    <TableCell className="armada-pool-slots__mark">
      <Tooltip label={mark.said}>
        <span
          role="img"
          aria-label={mark.said}
          data-tone={mark.tone}
          data-spins={mark.spins === true ? "" : undefined}
        >
          <mark.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </TableCell>
  );
}

function Holder({ slot, onOpenJob }: { slot: WorktreeSlot; onOpenJob: (jobId: string) => void }) {
  const held = slot.held;
  if (held.state === "job") {
    return (
      <button type="button" className="armada-pool-slots__job" onClick={() => onOpenJob(held.job_id)}>
        <Box size={12} strokeWidth={2} aria-hidden />
        <span>{held.job_title ?? held.job_id}</span>
      </button>
    );
  }
  if (held.state === "session") return <>{held.holder}</>;
  return null;
}

export function PoolSlots({ rows, onOpenJob }: PoolSlotsProps) {
  return (
    <Table className="armada-pool-slots">
      <TableHead>
        <TableRow>
          <TableHeaderCell>Slot</TableHeaderCell>
          <TableHeaderCell>State</TableHeaderCell>
          <TableHeaderCell>Build</TableHeaderCell>
          <TableHeaderCell>Branch</TableHeaderCell>
          <TableHeaderCell>Held by</TableHeaderCell>
          <TableHeaderCell>Held for</TableHeaderCell>
          <TableHeaderCell>{`Behind ${rows[0]?.slot.base ?? "base"}`}</TableHeaderCell>
        </TableRow>
      </TableHead>
      <TableBody>
        {rows.map(({ slot, heldFor }) => {
          const made = slot.held.state !== "unmade" && slot.held.state !== "not_a_checkout";
          const warmth: Mark | null = !made
            ? null
            : slot.warm
              ? { Glyph: Flame, said: "Warm", tone: "warm" }
              : { Glyph: Snowflake, said: "Cold", tone: "cold" };
          const state = stateMark(slot);
          return (
            <TableRow key={`${slot.manifest_id}/${slot.slot}`} data-tone={state.tone}>
              <TableCell variant="mono">
                <Tooltip label={slot.path}>
                  <span>{`slot-${slot.slot}`}</span>
                </Tooltip>
              </TableCell>
              <MarkCell mark={state} />
              <MarkCell mark={warmth} />
              <TableCell variant="mono">{slot.branch}</TableCell>
              <TableCell>
                <Holder slot={slot} onOpenJob={onOpenJob} />
              </TableCell>
              <TableCell variant="mono">{heldFor}</TableCell>
              <TableCell variant="mono">{slot.behind}</TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}
