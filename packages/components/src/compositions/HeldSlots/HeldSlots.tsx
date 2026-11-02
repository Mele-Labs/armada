import { useId, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { KeyRound } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { ageOf } from "../../time-format";

/** Why Free is off for a slot: its holder is running, or holds work no commit has. A committed branch is safe, pushed or not. */
export type HeldKeeps = "running" | "uncommitted";

export type HeldSlotRow = {
  slot: number;
  /** The Job that holds it. Its name opens it. */
  job: { id: string; title: string };
  /** The instant the holder last did anything, as RFC 3339. */
  quietSince: string;
  /** Absent where Free is on. */
  keeps?: HeldKeeps;
};

export type HeldSlotsProps = {
  rows: readonly HeldSlotRow[];
  /** Opens a holder's Job. */
  onOpen: (jobId: string) => void;
  /** Gives the slot to the Job the call is on. A promise keeps the row's Free pending, and every other off, until it settles. */
  onFree: (slot: number) => void | Promise<unknown>;
  /** Makes one more slot for the Job the call is on, as the last row. Absent draws no such row. */
  onCreate?: () => void | Promise<unknown>;
  now?: number;
};

const WHY: Record<HeldKeeps, string> = { running: "Running now", uncommitted: "Uncommitted work" };

const plural = (n: number, unit: string) => `${n} ${unit}${n === 1 ? "" : "s"}`;

/** The span the tooltip reads, to the hour: a bare `12d ago` has the figure behind it. */
function quietWords(since: string, now: number): string {
  const minutes = Math.floor(Math.max(0, now - Date.parse(since)) / 60_000);
  return minutes < 60 ? plural(minutes, "minute") : plural(Math.floor(minutes / 60), "hour");
}

/**
 * The slots a full pool is held by, drawn inside the call that offers to wait: one row each, the longest quiet first, with the Job that
 * holds it and how long it has been quiet, and Free at the end, then a last row that creates a slot. Free hands the slot to the waiting Job; the holder stays in the
 * list and leases another when it next writes. Free is off, with its reason on hover, where the holder is running or has uncommitted work,
 * and the column stays drawn so the rows line up.
 *
 * **The list is one Tab stop.** Up and Down move between rows, Return frees the row, `o` opens its Job. The keys stop here, so the
 * call's own Return and arrows do not also act.
 */
export function HeldSlots({ rows, onOpen, onFree, onCreate, now = Date.now() }: HeldSlotsProps) {
  const label = useId();
  const [at, setAt] = useState(0);
  const [out, setOut] = useState<number>();
  const [creating, setCreating] = useState(false);
  const items = useRef<(HTMLLIElement | null)[]>([]);
  const ordered = [...rows].sort((a, b) => Date.parse(a.quietSince) - Date.parse(b.quietSince));
  const last = ordered.length - (onCreate === undefined ? 1 : 0);
  const here = Math.min(at, last);
  const go = (to: number) => {
    const next = Math.min(last, Math.max(0, to));
    setAt(next);
    items.current[next]?.focus();
  };
  const free = (row: HeldSlotRow) => {
    if (out !== undefined || row.keeps !== undefined) return;
    const sent = onFree(row.slot);
    if (sent === undefined) return;
    setOut(row.slot);
    void sent.then(() => undefined, () => undefined).finally(() => setOut(undefined));
  };
  const create = () => {
    if (creating || out !== undefined) return;
    const sent = onCreate?.();
    if (sent === undefined) return;
    setCreating(true);
    void sent.then(() => undefined, () => undefined).finally(() => setCreating(false));
  };
  const key = (event: KeyboardEvent<HTMLLIElement>, index: number, row?: HeldSlotRow) => {
    // Return and `o` are the row's own: on its Free or its Job they are that button's press.
    const own = event.target === event.currentTarget;
    if (event.key === "ArrowDown") go(index + 1);
    else if (event.key === "ArrowUp") go(index - 1);
    else if (event.key === "Home") go(0);
    else if (event.key === "End") go(last);
    else if (own && event.key === "Enter") (row === undefined ? create() : free(row));
    else if (own && event.key === "o" && row !== undefined) onOpen(row.job.id);
    else return;
    event.preventDefault();
    event.nativeEvent.stopPropagation();
  };
  if (ordered.length === 0 && onCreate === undefined) return null;
  return (
    <section className="armada-held-slots" aria-labelledby={label}>
      <h3 id={label} className="armada-held-slots__eyebrow">
        Slots
      </h3>
      <ul className="armada-held-slots__rows">
        {ordered.map((row, index) => {
          const quiet = quietWords(row.quietSince, now);
          return (
            <li
              key={row.slot}
              ref={(element) => void (items.current[index] = element)}
              className="armada-held-slots__row"
              tabIndex={index === here ? 0 : -1}
              aria-label={`Slot ${row.slot}, ${row.job.title}, quiet for ${quiet}${row.keeps === undefined ? "" : `, Free is off: ${WHY[row.keeps]}`}`}
              onFocus={(event) => event.target === event.currentTarget && setAt(index)}
              onKeyDown={(event) => key(event, index, row)}
            >
              <Tooltip label={`Worktree slot ${row.slot}`}>
                <span className="armada-ref-chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
                  <KeyRound size={12} strokeWidth={2} aria-hidden />
                  {row.slot}
                </span>
              </Tooltip>
              <Tooltip label="Open Job" shortcut="o">
                <button type="button" className="armada-held-slots__job" tabIndex={-1} onClick={() => onOpen(row.job.id)}>
                  {row.job.title}
                </button>
              </Tooltip>
              <Tooltip label={`Quiet for ${quiet}`}>
                <span className="armada-held-slots__quiet">{ageOf(Date.parse(row.quietSince), now)}</span>
              </Tooltip>
              <Tooltip label={row.keeps === undefined ? "Gives the slot to the waiting Job. The holder leases another when it next writes." : WHY[row.keeps]} {...(row.keeps === undefined ? { shortcut: "↵" } : {})}>
                <span className="armada-held-slots__act">
                  <Button
                    size="sm"
                    variant="secondary"
                    tabIndex={-1}
                    aria-label={`Free slot ${row.slot}`}
                    disabled={row.keeps !== undefined || creating || (out !== undefined && out !== row.slot)}
                    pending={out === row.slot}
                    onClick={() => free(row)}
                  >
                    {out === row.slot ? "Freeing" : "Free"}
                  </Button>
                </span>
              </Tooltip>
            </li>
          );
        })}
        {onCreate === undefined ? null : (
          <li
            ref={(element) => void (items.current[ordered.length] = element)}
            className="armada-held-slots__row"
            tabIndex={here === ordered.length ? 0 : -1}
            aria-label="New slot"
            onFocus={(event) => event.target === event.currentTarget && setAt(ordered.length)}
            onKeyDown={(event) => key(event, ordered.length)}
          >
            <span className="armada-ref-chip" aria-hidden>
              <KeyRound size={12} strokeWidth={2} />+
            </span>
            <span className="armada-held-slots__new">New slot</span>
            <span />
            <Tooltip label="Creates a slot for this Job" shortcut="↵">
              <span className="armada-held-slots__act">
                <Button size="sm" variant="secondary" tabIndex={-1} aria-label="Create a slot" disabled={out !== undefined} pending={creating} onClick={create}>
                  {creating ? "Creating" : "Create"}
                </Button>
              </span>
            </Tooltip>
          </li>
        )}
      </ul>
    </section>
  );
}
