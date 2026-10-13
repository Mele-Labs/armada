import type { ReactNode } from "react";
import { Clock, ExternalLink, GitMerge, GitPullRequest } from "lucide-react";

import { Badge } from "../../primitives/Badge/Badge";
import { Button } from "../../primitives/Button/Button";
import { Prose } from "../../primitives/Prose/Prose";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { PlaceRow } from "../MergeLine/MergeLine";
import { PullRequestActs, type PullRequestActsProps } from "../PullRequestActs/PullRequestActs";
import { PullRequestPlates, type PullRequestChecks } from "./PullRequestPlates";

/**
 * A pull request as a panel docked to the trailing edge, with its status, its merge queue and its
 * failing checks as plates floating off its leading edge. The head is Sheet's: the pull request's
 * number names the dialog, its repository sits under it, and Close ends it. Below the head, the
 * title, the state, the branch and the acts, with Open in browser at the row's trailing end.
 *
 * **Drawn only from what Fleet serves** (`PullRequestState`, the merge line): no tabs, comment box,
 * approvals, conflicts, estimates, authors or per-check timings, and an absent fact leaves its slot
 * empty. The description is the last thing in the body, drawn as Markdown. At the window floor the plates are the panel's own first rows, since nothing is left
 * beside it.
 */
export type PullRequestPanelProps = {
  number: number;
  title: string;
  branch: string;
  /** Where it is on the forge. Open in browser takes it. */
  address: string;
  /** `owner/name`, where the address names it. */
  repository?: string;
  state: "draft" | "open" | "merged";
  checks: PullRequestChecks;
  auto: boolean;
  queued?: boolean;
  /** The merge line it is in, from `placeOf`. Absent draws no queue plate. */
  place?: readonly PlaceRow[];
  /** The window is at `--window-floor`: the plates move into the panel. */
  floor?: boolean;
  /** The act pressed that Fleet has not answered yet. */
  pending?: PullRequestActsProps["pending"];
  reviewed?: boolean;
  onAct: PullRequestActsProps["onAct"];
  /**
   * Tells the Session the panel was opened from that its checks failed. Absent where no Session
   * is the owner of the pull request, which draws no such act.
   */
  onSendBack?: () => void;
  /** The Session has been told, for these failing checks. */
  sentBack?: boolean;
  onOpenLink: (address: string) => void;
  onClose: () => void;
  /** What its author wrote of it, as Markdown. Absent or blank draws nothing. */
  body?: string;
  /** What Fleet refused, or anything else the host holds for the body. */
  children?: ReactNode;
};

const PILL = {
  merged: { status: "completed-success", Icon: GitMerge, said: "Merged" },
  queued: { status: "running", Icon: Clock, said: "In merge queue" },
  draft: { status: "not-started", Icon: GitPullRequest, said: "Draft" },
  open: { status: "completed-success", Icon: GitPullRequest, said: "Open" },
} as const;

export function PullRequestPanel({
  number,
  title,
  branch,
  address,
  repository,
  state,
  checks,
  auto,
  queued = false,
  place,
  floor = false,
  pending,
  reviewed = false,
  onAct,
  onSendBack,
  sentBack = false,
  onOpenLink,
  onClose,
  body,
  children,
}: PullRequestPanelProps) {
  const pill = PILL[state === "merged" ? "merged" : queued ? "queued" : state];
  const plates = <PullRequestPlates state={state} checks={checks} auto={auto} queued={queued} {...(place === undefined ? {} : { place })} onOpenPullRequest={onOpenLink} />;
  const description = body?.trim() ?? "";
  const sendBack = onSendBack !== undefined && checks.state === "failed" && state !== "merged";
  return (
    <Sheet
      kind="pull-request"
      open
      floating
      floor={floor}
      title={`Pull request #${number}`}
      {...(repository === undefined ? {} : { subtitle: <span className="armada-pr-panel__repository mono">{repository}</span> })}
      leading={<GitPullRequest size={16} strokeWidth={2} aria-hidden />}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      aside={<div className="armada-pr-plates">{plates}</div>}
    >
      <div className="armada-pr-panel">
        <h3 className="armada-pr-panel__title">{title}</h3>
        <div className="armada-pr-panel__meta">
          <Badge status={pill.status} icon={pill.Icon}>
            {pill.said}
          </Badge>
          <Tooltip label="Branch">
            <span className="armada-pr-panel__branch mono">{branch}</span>
          </Tooltip>
        </div>
        <div className="armada-pr-panel__acts" role="group" aria-label="Acts">
          {sendBack ? (
            <Tooltip label="Tells the Session which checks failed">
              <Button size="sm" variant="primary" disabled={sentBack} onClick={onSendBack}>
                {sentBack ? "Sent back" : "Send back to Session"}
              </Button>
            </Tooltip>
          ) : null}
          <PullRequestActs state={state} checks={checks.state} auto={auto} queued={queued} {...(pending === undefined ? {} : { pending })} reviewed={reviewed} onAct={onAct} />
          <span className="armada-pr-panel__open">
            <Tooltip label="Open in browser">
              <Button variant="ghost" size="sm" iconOnly aria-label="Open in browser" onClick={() => onOpenLink(address)}>
                <ExternalLink size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        </div>
        {floor ? <div className="armada-pr-plates">{plates}</div> : null}
        {children}
        {description === "" ? null : (
          <div className="armada-pr-panel__description">
            <Prose text={description} />
          </div>
        )}
      </div>
    </Sheet>
  );
}
