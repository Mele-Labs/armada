// What hovering a merge line chip shows: the Session ledger's card (`LedgerPreview`), built from the chip's dot, so a pull request
// is said the same way in the ledger and on the line. Only facts the dot holds; a teammate's quiet chip holds few, and none that call for him.
// The Job row is a link to the Job where one in this window holds the pull request, and absent where none does.

import { GitBranch, GitPullRequest } from "lucide-react";
import { LedgerPreview } from "@armada/components";
import type { SessionLedgerCard, SessionLedgerTone } from "@armada/components";

import { MERGE_BY_HAND, type Dot, type PullState } from "../horizon";

const TONE: Partial<Record<PullState, SessionLedgerTone>> = { ready: "passed", failing: "failed", running: "running", blocked: "escalated" };

const REVIEW = { approved: "Approved", changes: "Changes requested", required: "Required" } as const;

export function chipCardOf(dot: Dot, openJob?: (id: string) => void): SessionLedgerCard {
  const { card } = dot;
  const tone = TONE[dot.state];
  const pull = card.heading.startsWith("#");
  return {
    title: card.title ?? card.heading,
    where: card.heading,
    state: { said: card.draft === true ? `${card.says}, draft` : card.says, Glyph: pull ? GitPullRequest : GitBranch, ...(tone === undefined ? {} : { tone }) },
    facts: [
      ...(card.author === undefined ? [] : [{ label: "Author", value: card.author }]),
      ...(card.check === undefined ? [] : [{ label: "Checks", value: card.check }]),
      ...(card.place === undefined ? [] : [{ label: pull ? "Place" : "Turn", value: String(card.place) }]),
      ...(card.branch === card.heading ? [] : [{ label: "Branch", value: card.branch, mono: true }]),
      ...(card.review === undefined ? [] : [{ label: "Review", value: REVIEW[card.review] }]),
      ...(card.conflicts === true ? [{ label: "Main", value: "Conflicts", tone: "failed" as const }] : []),
      ...(card.byHand === true ? [{ label: "Merge", value: MERGE_BY_HAND }] : []),
      ...(card.owner === undefined ? [] : [{ label: "Job", value: card.owner.title, ...(openJob === undefined ? {} : { open: () => openJob(card.owner!.id) }) }]),
    ],
  };
}

/** The chip's hover, for `Inspect`: a pull request's chip selects it on a press, so the card says so. Opening the Job puts the card away. */
export const chipPreview =
  (dot: Dot, pressable: boolean, openJob?: (id: string) => void) =>
  ({ close }: { close: () => void }) => (
    <LedgerPreview
      card={chipCardOf(dot, openJob === undefined ? undefined : (id) => (close(), openJob(id)))}
      kind={dot.card.heading.startsWith("#") ? "pull_request" : "branch"}
      keys={pressable ? [{ key: "Enter", verb: "Open" }] : []}
    />
  );
