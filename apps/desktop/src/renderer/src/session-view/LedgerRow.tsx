// The one row a Job's lists draw: Landing's pull requests and every row of the Artifacts layer. Two lines. The first is a glyph and the title, which wraps to
// a second line before it is cut. The second is small and muted: the branch or path, then the facts a person reads off it, each a glyph with its tooltip and a
// word beside it. An act, where the row has one, stands in a column of its own at the right, centred on the row and kept where a row has none. The rows are
// divided by a hairline; there is no card inside the panel.

import { useState, type ReactNode } from "react";
import { Clock, GitMerge, GitPullRequest, GitPullRequestArrow, Link, type LucideIcon } from "lucide-react";
import { Button, PullRequestChecksMark, Tooltip } from "@armada/components";
import type { PullRequestAct, Session } from "@armada/screens/src/draft/sessions";

import { usePullRequestMenu } from "../pull-request-menu";
import { checksSaid } from "../session-cards";
import { useSessionsDraft } from "../sessions-draft";
import { checksOf, openPrPanel, type PullRequest } from "./IssuesBlock";
import "./ledger-row.css";

/** A glyph that names itself on hover. */
export function Glyphed({ Glyph, said, state }: { Glyph: LucideIcon; said: string; state?: string }) {
  return (
    <Tooltip asChild label={said}>
      <span className="sv-ledger__glyph" role="img" aria-label={said} data-state={state}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** A fact on the second line: its glyph, which carries the tooltip, and its word. */
export function Fact({ Glyph, said, word, state }: { Glyph: LucideIcon; said: string; word: string; state?: string }) {
  return (
    <span className="sv-ledger__fact" data-state={state}>
      <Glyphed Glyph={Glyph} said={said} {...(state === undefined ? {} : { state })} />
      {word}
    </span>
  );
}

type LedgerRowProps = {
  /** The first line's lead: the kind's glyph, or a pull request's state. */
  lead: ReactNode;
  /** A number or key before the title, in mono. */
  num?: string;
  title: string;
  /** The second line. */
  meta: ReactNode;
  /** The act: a node draws it, `null` keeps its column empty, absent gives the row no column. */
  act?: ReactNode;
  /** Absent, the row has nowhere to go and is read. */
  onOpen?: (() => void) | undefined;
  label: string;
  /** Handlers a context menu hangs on the row. */
  on?: object;
  /** What the context menu draws, kept inside the row. */
  menu?: ReactNode;
  /** The artifact kind, which gives the row's glyph its hue. */
  kind?: string;
  /** A pull request's state, which frames the row in its hue: open, draft, merged, closed, or failed where its Checks refused. */
  tone?: string;
  /** A stacked pull request's lane: a gutter beside the row whose line joins the rows of one stack. */
  lane?: { first: boolean; last: boolean };
  /** Names the row's pull request number, so a chip elsewhere can find it. */
  anchor?: number;
};

export function LedgerRow({ lead, num, title, meta, act, onOpen, label, on, menu, kind, tone, lane, anchor }: LedgerRowProps) {
  const inside = (
    <>
      {lead}
      <span className="sv-ledger__title">
        {num === undefined ? null : <code className="sv-ledger__num">{num}</code>}
        {title}
      </span>
      <span className="sv-ledger__meta">{meta}</span>
    </>
  );
  const body = (
    <>
      {onOpen === undefined ? (
        <div className="sv-ledger__main" data-plain="">
          {inside}
        </div>
      ) : (
        <button {...on} type="button" className="sv-ledger__main" data-layer-row="" aria-label={label} onClick={onOpen}>
          {inside}
        </button>
      )}
      {act === undefined ? null : <span className="sv-ledger__act">{act}</span>}
      {menu}
    </>
  );
  const data = { "data-acts": act === undefined ? undefined : "", "data-kind": kind, "data-tone": tone };
  // A stacked row is the lane beside a card, so the item holds both and the card is no longer the list item.
  // The stack's root stands flush left and the line starts under it; only what is stacked on it is drawn in from the lane.
  if (lane === undefined) return <li className="sv-ledger" data-pr={anchor} {...data}>{body}</li>;
  return (
    <li className="sv-stack-item" data-pr={anchor} data-root={lane.first ? "" : undefined}>
      {lane.first ? null : <span className="sv-lane" aria-hidden="true" data-last={lane.last ? "" : undefined} />}
      <div className="sv-ledger" {...data}>{body}</div>
    </li>
  );
}

/** Where the pull request stands on its way in: a draft, the merge queue or auto-merge. A merged one says so by its lead glyph and its frame alone. */
function standingOf(pr: PullRequest): { Glyph: LucideIcon; said: string; word: string; state?: string } | null {
  if (pr.state === "draft") return { Glyph: GitPullRequest, said: "Draft, not ready for review", word: "Draft", state: "draft" };
  if (pr.queued === true) return { Glyph: Clock, said: "In the merge queue, merges when its turn comes", word: "Queued", state: "queued" };
  if (pr.checks.state === "pending" && pr.auto) return { Glyph: GitMerge, said: "Auto-merge on, merges when every Check has passed", word: "Auto-merge", state: "queued" };
  return null;
}

/** The icon button that moves a pull request on, where one fits. */
function actOf(pr: PullRequest, pending: PullRequestAct | undefined, press: (act: PullRequestAct) => void): ReactNode {
  const button = (which: PullRequestAct, Glyph: LucideIcon, name: string, said: string) => (
    <Tooltip label={said}>
      <Button variant="ghost" size="sm" iconOnly aria-label={`${name} #${pr.number}`} pending={pending === which} disabled={pending !== undefined} onClick={() => press(which)}>
        <Glyph size={16} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
  if (pr.state === "merged" || pr.queued === true) return null;
  if (pr.state === "draft") return button("ready", GitPullRequestArrow, "Ready for review", "Marks the pull request ready for review");
  if (pr.checks.state === "passed") return button("merge", GitMerge, "Merge", "Merges it now, every Check has passed");
  if (pr.checks.state === "pending" && !pr.auto) return button("auto_merge", GitMerge, "Enable auto-merge", "Enable auto-merge, merges it when every Check has passed");
  return null;
}

const CHECK_WORD = { passed: "Passed", running: "Running", failed: "Failed" } as const;

/** Names the pull request this one sits on. It lives inside the row's own button, so it is a link role, not a second button. */
function StackChip({ parent, onJump }: { parent: number; onJump: ((number: number) => void) | undefined }) {
  const go = (event: { stopPropagation: () => void }) => {
    event.stopPropagation();
    onJump?.(parent);
  };
  return (
    <span
      className="sv-ledger__on"
      role="link"
      tabIndex={0}
      aria-label={`Stacked on pull request #${parent}`}
      onClick={go}
      onKeyDown={(event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        go(event);
      }}
    >
      <Link size={12} strokeWidth={2} aria-hidden />
      on #{parent}
    </span>
  );
}

/** A pull request as a row, in Landing and in the Artifacts layer alike. */
export function PullRow({ session, pr, from, lane, rootBase, chip, onJump }: { session: Session; pr: PullRequest; /** The Job the pull request came from, where it is not the Job whose list this is. */ from?: string; lane?: { first: boolean; last: boolean }; rootBase?: string; chip?: number; onJump?: (number: number) => void }) {
  const draft = useSessionsDraft();
  const [pending, setPending] = useState<PullRequestAct | undefined>(undefined);
  const { on, menu } = usePullRequestMenu(session.manifestId, pr.number, pr.title, pr.address);
  const press = (act: PullRequestAct) => {
    if (draft === undefined) return;
    setPending(act);
    void draft.act(session.id, pr.number, act).finally(() => setPending(undefined));
  };
  const checks = checksOf(pr);
  const standing = standingOf(pr);
  return (
    <LedgerRow
      tone={pr.state !== "merged" && pr.state !== "draft" && checks === "failed" ? "failed" : pr.state}
      lead={<Glyphed Glyph={pr.state === "merged" ? GitMerge : GitPullRequest} said={pr.state === "merged" ? "Merged" : pr.state === "draft" ? "Draft" : "Open"} state={pr.state} />}
      num={`#${pr.number}`}
      title={pr.title}
      label={`Pull request #${pr.number} ${pr.title}`}
      onOpen={() => openPrPanel(pr)}
      on={on}
      menu={menu}
      act={actOf(pr, pending, press)}
      anchor={pr.number}
      {...(lane === undefined ? {} : { lane })}
      meta={
        <>
          {rootBase === undefined ? null : <span className="sv-ledger__word" data-base="">base: {rootBase}</span>}
          {chip === undefined ? null : <StackChip parent={chip} onJump={onJump} />}
          {from === undefined ? null : <span className="sv-ledger__word">{from}</span>}
          <code className="sv-ledger__sub">{pr.branch}</code>
          <span className="sv-ledger__fact" data-state={checks}>
            <PullRequestChecksMark checks={checks} said={checksSaid(pr.checks)} />
            {CHECK_WORD[checks]}
          </span>
          {standing === null ? null : <Fact {...standing} />}
        </>
      }
    />
  );
}
