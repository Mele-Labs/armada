import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import { GitBranch, GitPullRequest, KeyRound, ShieldCheck, ShieldEllipsis, ShieldX, SquareTerminal } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Inspect, InspectCard } from "../../primitives/Inspect/Inspect";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { sessionGlyphOf } from "../SessionFrame/SessionFrame";
import type { SessionState } from "../SessionFrame/SessionFrame";

/**
 * A branch, pull request or slot named anywhere in Bridge, and who owns it.
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
  | { kind: "slot"; slot: number };

/** What the card says of the owner: its title, slot, pull requests and last turn. */
export type OwnerSummary = {
  id: string;
  /** What other sessions call it, drawn where the id would be. */
  address?: string;
  title?: string;
  state: SessionState;
  /** What the state's mark says on hover. */
  said: string;
  slots: readonly number[];
  pullRequests: readonly { number: number; checks: "pending" | "passed" | "failed"; said: string; red?: RedChecks }[];
  /** When its last turn ended, an RFC 3339 instant. */
  lastTurnAt?: string;
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
  }
};

const GLYPH: Record<OwnerChipRef["kind"], LucideIcon> = {
  branch: GitBranch,
  pull_request: GitPullRequest,
  slot: KeyRound,
};

const TEXT = (chip: OwnerChipRef): string => {
  switch (chip.kind) {
    case "branch":
      return chip.name;
    case "pull_request":
      return `#${chip.number}`;
    case "slot":
      return String(chip.slot);
  }
};

/** A check red that does not decide the pull request: while the run goes on, or after the required ones passed. */
export type RedChecks = "running" | "advisory";

const CHECKS: Record<"pending" | "passed" | "failed", LucideIcon> = { pending: ShieldEllipsis, passed: ShieldCheck, failed: ShieldX };

/**
 * A chip drawn whole: its glyph and its figure, named for what it is. Never a bare figure.
 * **A pull request's Checks sit inside it**, at its right end past a divider, coloured by how they stand.
 */
export function Chip({ chip, matched = false, checks }: { chip: OwnerChipRef; matched?: boolean; checks?: { state: "pending" | "passed" | "failed"; said: string; red?: RedChecks } }) {
  const Glyph = GLYPH[chip.kind];
  const Shield = checks === undefined || chip.kind !== "pull_request" ? undefined : CHECKS[checks.state];
  return (
    <span className="armada-ref-chip" data-matched={matched || undefined} role="img" aria-label={`${matched ? "Matched " : ""}${NAME(chip)}${Shield === undefined || checks === undefined ? "" : `, ${checks.said}`}`}>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      {TEXT(chip)}
      {Shield === undefined || checks === undefined ? null : (
        <Tooltip label={checks.said}>
          <span className="armada-ref-chip__checks" data-checks={checks.state} data-red={checks.red}>
            <Shield size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
    </span>
  );
}

/** The tone a Session's state is drawn in, the status tokens the rest of Bridge reads. */
const STATE_TONE: Partial<Record<SessionState, string>> = {
  working: "var(--status-running)",
  waiting: "var(--status-awaiting-review)",
  replied: "var(--status-awaiting-review)",
  stuck: "var(--status-awaiting-review)",
  failing: "var(--status-completed-failed)",
  done: "var(--status-completed-success)",
  idle: "var(--status-completed-success)",
};

/** The card: the Session as the inspect card says it, with what it holds and the way into it. */
export function OwnerCard({ owner, onOpen }: { owner: OwnerSummary; onOpen: () => void }) {
  const tone = STATE_TONE[owner.state];
  return (
    <InspectCard
      kind={{ Glyph: SquareTerminal, label: "Job" }}
      where={owner.address ?? owner.id}
      title={owner.title}
      state={{ Glyph: sessionGlyphOf(owner.state), said: owner.said, ...(tone === undefined ? {} : { tone }) }}
      facts={owner.lastTurnAt === undefined ? [] : [{ label: "Last turn", value: <Stamp at={owner.lastTurnAt} />, mono: true }]}
      keys={[{ key: "Enter", verb: "Keep open" }]}
    >
      <ul className="armada-owner-card__facts">
        {owner.slots.map((slot) => (
          <li key={slot}>
            <Chip chip={{ kind: "slot", slot }} />
          </li>
        ))}
        {owner.pullRequests.map((pr) => (
          <li key={pr.number}>
            <Chip chip={{ kind: "pull_request", number: pr.number }} checks={{ state: pr.checks, said: pr.said, ...(pr.red === undefined ? {} : { red: pr.red }) }} />
          </li>
        ))}
      </ul>
      <Button size="sm" variant="secondary" onClick={onOpen}>
        Open Job
      </Button>
    </InspectCard>
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
  if (owner === undefined || ownership === null) return <>{plain ?? children}</>;
  return (
    <span className="armada-owner-chip">
      <Inspect card={<OwnerCard owner={owner} onOpen={() => ownership.open(owner.id)} />} name={`Owned by ${owner.title ?? owner.address ?? owner.id}`}>
        {/* A press keeps the card; it must not also reach a row that opens on press. */}
        <button type="button" className="armada-owner-chip__press" aria-label={NAME(chip)} onClick={(event) => event.stopPropagation()}>
          {children}
        </button>
      </Inspect>
    </span>
  );
}
