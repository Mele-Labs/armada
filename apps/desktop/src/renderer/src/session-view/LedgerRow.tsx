// The one row a Job's lists draw: Landing's pull requests and every row of the Artifacts layer. Two lines. The first is a glyph and the title, which wraps to
// a second line before it is cut. The second is small and muted: the branch or path, then the facts a person reads off it, each a glyph with its tooltip and a
// word beside it. An act, where the row has one, stands in a column of its own at the right, centred on the row and kept where a row has none. The rows are
// divided by a hairline; there is no card inside the panel.

import { useState, type ReactNode } from "react";
import { Clock, GitMerge, GitPullRequest, GitPullRequestArrow, type LucideIcon } from "lucide-react";
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
};

export function LedgerRow({ lead, num, title, meta, act, onOpen, label, on, menu }: LedgerRowProps) {
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
  return (
    <li className="sv-ledger" data-acts={act === undefined ? undefined : ""}>
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
    </li>
  );
}

/** The merge queue or the merge, where the pull request has one. */
function standingOf(pr: PullRequest): { Glyph: LucideIcon; said: string; word: string; state?: string } | null {
  if (pr.state === "merged") return { Glyph: GitMerge, said: "Merged", word: "Merged", state: "merged" };
  if (pr.state === "draft") return { Glyph: GitPullRequest, said: "Draft, not ready for review", word: "Draft" };
  if (pr.queued === true) return { Glyph: Clock, said: "In the merge queue, merges when its turn comes", word: "Queued" };
  if (pr.checks.state === "pending" && pr.auto) return { Glyph: GitMerge, said: "Auto-merge on, merges when every Check has passed", word: "Auto-merge" };
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

/** A pull request as a row, in Landing and in the Artifacts layer alike. */
export function PullRow({ session, pr }: { session: Session; pr: PullRequest }) {
  const draft = useSessionsDraft();
  const [pending, setPending] = useState<PullRequestAct | undefined>(undefined);
  const { on, menu } = usePullRequestMenu(session.manifestId, pr.number, pr.title);
  const press = (act: PullRequestAct) => {
    if (draft === undefined) return;
    setPending(act);
    void draft.act(session.id, pr.number, act).finally(() => setPending(undefined));
  };
  const checks = checksOf(pr);
  const standing = standingOf(pr);
  return (
    <LedgerRow
      lead={<Glyphed Glyph={pr.state === "merged" ? GitMerge : GitPullRequest} said={pr.state === "merged" ? "Merged" : pr.state === "draft" ? "Draft" : "Open"} state={pr.state} />}
      num={`#${pr.number}`}
      title={pr.title}
      label={`Pull request #${pr.number} ${pr.title}`}
      onOpen={() => openPrPanel(pr)}
      on={on}
      menu={menu}
      act={actOf(pr, pending, press)}
      meta={
        <>
          <code className="sv-ledger__sub">{pr.branch}</code>
          <span className="sv-ledger__fact">
            <PullRequestChecksMark checks={checks} said={checksSaid(pr.checks)} />
            {CHECK_WORD[checks]}
          </span>
          {standing === null ? null : <Fact {...standing} />}
        </>
      }
    />
  );
}
