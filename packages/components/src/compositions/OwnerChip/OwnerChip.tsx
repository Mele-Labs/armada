import { createContext, useContext, useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import { Box, GitBranch, GitPullRequest, KeyRound, ShieldCheck, ShieldEllipsis, ShieldX, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SessionMark } from "../SessionFrame/SessionFrame";
import type { SessionState } from "../SessionFrame/SessionFrame";

/**
 * A branch, pull request, slot or Job named anywhere in Bridge, and who owns it.
 *
 * **Where a Session owns what a chip names, hovering the chip draws that
 * Session's card**, and pressing the chip keeps the card up until Escape or a
 * press elsewhere. Where nothing owns it, or no Sessions are served at all, the
 * chip draws exactly as it did: `plain`, or its children. A host provides who
 * owns what through `ChipOwnership`; a surface that draws a chip never learns
 * what a Session is.
 */
export type OwnerChipRef =
  | { kind: "branch"; name: string }
  | { kind: "pull_request"; number: number }
  | { kind: "slot"; slot: number }
  | { kind: "job"; id: string; number: number };

/** What the card says of the owner: its title, slot, pull requests, Jobs and last turn. */
export type OwnerSummary = {
  id: string;
  title?: string;
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  slots: readonly number[];
  pullRequests: readonly { number: number; checks: "pending" | "passed" | "failed"; said: string }[];
  jobs: readonly { id: string; number: number }[];
  lastTurn?: string;
};

export type ChipOwnershipValue = {
  ownerOf: (chip: OwnerChipRef) => OwnerSummary | undefined;
  open: (sessionId: string) => void;
};

/** What the window says about owners. Absent, no chip is owned. */
export const ChipOwnership = createContext<ChipOwnershipValue | null>(null);

/** The owner of a chip, where the window serves Sessions and one owns it. */
export function useChipOwner(chip: OwnerChipRef): OwnerSummary | undefined {
  return useContext(ChipOwnership)?.ownerOf(chip);
}

const NAME = (chip: OwnerChipRef): string => {
  switch (chip.kind) {
    case "branch":
      return `Branch ${chip.name}`;
    case "pull_request":
      return `Pull request #${chip.number}`;
    case "slot":
      return `Worktree slot ${chip.slot}`;
    case "job":
      return `Job ${chip.number}`;
  }
};

const GLYPH: Record<OwnerChipRef["kind"], LucideIcon> = {
  branch: GitBranch,
  pull_request: GitPullRequest,
  slot: KeyRound,
  job: Box,
};

const TEXT = (chip: OwnerChipRef): string => {
  switch (chip.kind) {
    case "branch":
      return chip.name;
    case "pull_request":
      return `#${chip.number}`;
    case "slot":
      return String(chip.slot);
    case "job":
      return String(chip.number);
  }
};

/** A chip drawn whole: its glyph and its figure, named for what it is. Never a bare figure. */
export function Chip({ chip, matched = false }: { chip: OwnerChipRef; matched?: boolean }) {
  const Glyph = GLYPH[chip.kind];
  return (
    <span className="armada-ref-chip" data-matched={matched || undefined} role="img" aria-label={`${matched ? "Matched " : ""}${NAME(chip)}`}>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      {TEXT(chip)}
    </span>
  );
}

const CHECKS: Record<"pending" | "passed" | "failed", LucideIcon> = { pending: ShieldEllipsis, passed: ShieldCheck, failed: ShieldX };

/** The card: the Session's head, with its state's mark, over what it holds. */
export function OwnerCard({ owner, onOpen, at }: { owner: OwnerSummary; onOpen: () => void; at?: { top: number; left: number } }) {
  return (
    <div
      className="armada-owner-card"
      data-owner-chip
      role="group"
      aria-label={`Owned by ${owner.title ?? owner.id}`}
      style={at === undefined ? undefined : { top: at.top, left: at.left }}
    >
      <div className="armada-owner-card__band">
        <SquareTerminal size={12} strokeWidth={2} aria-hidden />
        <span className="armada-owner-card__id">{owner.id}</span>
        <span className="armada-owner-card__title">{owner.title}</span>
        <SessionMark state={owner.state} said={owner.said} />
      </div>
      <div className="armada-owner-card__body">
        <ul className="armada-owner-card__facts">
          {owner.slots.map((slot) => (
            <li key={slot}>
              <Chip chip={{ kind: "slot", slot }} />
            </li>
          ))}
          {owner.pullRequests.map((pr) => {
            const Glyph = CHECKS[pr.checks];
            return (
              <li key={pr.number}>
                <Chip chip={{ kind: "pull_request", number: pr.number }} />
                <Tooltip label={pr.said}>
                  <span role="img" aria-label={pr.said} className="armada-owner-card__checks">
                    <Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </li>
            );
          })}
          {owner.jobs.map((job) => (
            <li key={job.id}>
              <Chip chip={{ kind: "job", id: job.id, number: job.number }} />
            </li>
          ))}
        </ul>
        {owner.lastTurn === undefined ? null : (
          <Tooltip label="Last turn">
            <span className="armada-owner-card__last" aria-label={`Last turn ${owner.lastTurn}`}>
              {owner.lastTurn}
            </span>
          </Tooltip>
        )}
        <Button size="sm" variant="secondary" onClick={onOpen}>
          Open Session
        </Button>
      </div>
    </div>
  );
}

export type OwnerChipProps = {
  chip: OwnerChipRef;
  /** What is drawn, and pressed, when somebody owns the chip. */
  children: ReactNode;
  /** What is drawn where nobody does. Absent draws the children. */
  plain?: ReactNode;
};

export function OwnerChip({ chip, children, plain }: OwnerChipProps) {
  const ownership = useContext(ChipOwnership);
  const owner = ownership?.ownerOf(chip);
  const [hovered, setHovered] = useState(false);
  const [kept, setKept] = useState(false);
  const anchor = useRef<HTMLSpanElement>(null);
  const [where, setWhere] = useState<{ top: number; left: number } | undefined>();
  const shown = hovered || kept;
  useLayoutEffect(() => {
    const box = anchor.current?.getBoundingClientRect();
    setWhere(shown && box !== undefined ? { top: box.bottom, left: box.left } : undefined);
  }, [shown]);
  useEffect(() => {
    if (!kept) return;
    const away = (event: MouseEvent) => {
      if (!(event.target instanceof Element) || event.target.closest("[data-owner-chip]") === null) setKept(false);
    };
    const escape = (event: KeyboardEvent) => event.key === "Escape" && setKept(false);
    document.addEventListener("mousedown", away);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("mousedown", away);
      document.removeEventListener("keydown", escape);
    };
  }, [kept]);

  if (owner === undefined || ownership === null) return <>{plain ?? children}</>;
  const open = hovered || kept;
  return (
    <span
      ref={anchor}
      className="armada-owner-chip"
      data-owner-chip
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onFocus={() => setHovered(true)}
      onBlur={() => setHovered(false)}
    >
      <button
        type="button"
        className="armada-owner-chip__press"
        aria-label={NAME(chip)}
        aria-expanded={open}
        onClick={(event) => {
          event.stopPropagation();
          setKept((was) => !was);
        }}
      >
        {children}
      </button>
      {/* In the body, not beside the chip: a tile or a row clips what hangs off it. */}
      {open && where !== undefined
        ? createPortal(<OwnerCard owner={owner} at={where} onOpen={() => ownership.open(owner.id)} />, document.body)
        : null}
    </span>
  );
}
