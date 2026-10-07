import type { ReactNode } from "react";
import { Box, Check, CircleDot, GitBranch, GitPullRequest, KeyRound, Presentation, PencilRuler, ShieldCheck, ShieldEllipsis, ShieldX, Split } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a Session has accumulated, a section per kind and a row per thing.
 * **Every row is a press that opens what it names**: the slot on Cleanup, the
 * pull request in a sheet, the Job in its detail, the Studio on its whiteboard,
 * a sketch beside it. The ledger draws no surface of its own; the host says
 * where each press goes.
 *
 * **Every section is drawn from the start, dim, with an empty well under it and
 * no sentence in it**, so a blank Session's ledger shows what it will hold, and
 * each section fills in place when its first row arrives (the owner's choice
 * of 7 Oct 2026, on the empty ledger).
 */
export type LedgerKind = "slot" | "branch" | "pull_request" | "job" | "studio" | "sketch" | "subagent";

export type LedgerEntry = {
  key: string;
  kind: LedgerKind;
  /** The row's accessible name: what it is, and for a pull request how its Checks stand. */
  name: string;
  text: ReactNode;
  /** A pull request's Checks, or a subagent's turn, as the one mark at the row's end. */
  mark?: { glyph: "pending" | "passed" | "failed" | "running" | "done"; said: string };
  /** A Job's own slot, as a chip at the row's end. */
  slot?: number;
  onOpen: () => void;
};

const SECTIONS: { kind: LedgerKind; label: string; Glyph: LucideIcon }[] = [
  { kind: "slot", label: "Worktree Slot", Glyph: KeyRound },
  { kind: "branch", label: "Branches", Glyph: GitBranch },
  { kind: "pull_request", label: "Pull requests", Glyph: GitPullRequest },
  { kind: "job", label: "Jobs", Glyph: Box },
  { kind: "studio", label: "Studios", Glyph: Presentation },
  { kind: "sketch", label: "Sketches", Glyph: PencilRuler },
  { kind: "subagent", label: "Subagents", Glyph: Split },
];

const MARK: Record<NonNullable<LedgerEntry["mark"]>["glyph"], LucideIcon> = {
  pending: ShieldEllipsis,
  passed: ShieldCheck,
  failed: ShieldX,
  running: CircleDot,
  done: Check,
};

export function SessionLedger({ entries }: { entries: readonly LedgerEntry[] }) {
  return (
    <aside className="armada-session-ledger" role="region" aria-label="Attachments">
      {SECTIONS.map(({ kind, label, Glyph }) => {
        const rows = entries.filter((one) => one.kind === kind);
        return (
          <section className="armada-session-ledger__group" key={kind} aria-label={label} data-empty={rows.length === 0 || undefined}>
            <h3 className="armada-session-ledger__eyebrow">{label}</h3>
            <ul className="armada-session-ledger__rows">
              {rows.map((row) => {
                const Mark = row.mark === undefined ? undefined : MARK[row.mark.glyph];
                return (
                  <li key={row.key} className="armada-session-ledger__row" aria-label={row.name}>
                    <button type="button" className="armada-session-ledger__open" aria-label={`Open ${row.name}`} onClick={row.onOpen}>
                      <Glyph size={12} strokeWidth={2} aria-hidden />
                      <span className="armada-session-ledger__text">{row.text}</span>
                      {row.slot === undefined ? null : (
                        <span className="armada-ref-chip" role="img" aria-label={`Worktree slot ${row.slot}`}>
                          <KeyRound size={12} strokeWidth={2} aria-hidden />
                          {row.slot}
                        </span>
                      )}
                      {Mark === undefined || row.mark === undefined ? null : (
                        <Tooltip label={row.mark.said}>
                          <span
                            className="armada-session-mark"
                            role="img"
                            aria-label={row.mark.said}
                            data-pulsing={row.mark.glyph === "running" || undefined}
                          >
                            <Mark size={12} strokeWidth={2} aria-hidden />
                          </span>
                        </Tooltip>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </aside>
  );
}
