import { LoaderCircle } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a person does to a pull request without going to the browser: **Ready
 * for review** while it is a draft, **Merge** once every Check has passed,
 * **Enable auto-merge** while they are still running, and **Review**, which
 * dispatches a Job on the code review workflow against it.
 *
 * **Only the acts that fit are drawn.** A merged pull request has none, a red
 * one cannot be merged, and an act already asked for is drawn as the fact. Each
 * says what it does in its tooltip, since the verb alone does not.
 *
 * **A press Fleet has not answered is `pending`**: that button is Button's own
 * pending rendering, the bar travelling its edge, with a turning ring before
 * its label as the Fleet build button has, and the others are disabled so a
 * second act cannot start. The label does not change. Once Fleet has
 * accepted a Review the caller sets `reviewed`, and the button reads "Review
 * dispatched", disabled, since a second one would review it twice.
 */
type Act = "ready" | "merge" | "auto_merge" | "review";

export type PullRequestActsProps = {
  state: "draft" | "open" | "merged";
  checks: "pending" | "passed" | "failed";
  /** Auto-merge has been asked for. */
  auto: boolean;
  /** It is in the merge queue: the forge has it, so no act asks for that again. */
  queued?: boolean;
  /** The act pressed that Fleet has not answered yet. */
  pending?: Act;
  /** A Review has been dispatched against it. */
  reviewed?: boolean;
  onAct: (act: Act) => void;
};

export function PullRequestActs({ state, checks, auto, queued = false, pending, reviewed = false, onAct }: PullRequestActsProps) {
  if (state === "merged") return null;
  const busy = pending !== undefined;
  const turning = (act: Act) => (pending === act ? <LoaderCircle className="armada-pr-acts__spin" size={16} strokeWidth={2} aria-hidden="true" /> : null);
  return (
    <div className="armada-pr-acts" role="group" aria-label="Pull request acts">
      {state === "draft" ? (
        <Tooltip label="Marks the pull request ready for review">
          <Button size="sm" variant="primary" pending={pending === "ready"} disabled={busy} onClick={() => onAct("ready")}>
            {turning("ready")}
            Ready for review
          </Button>
        </Tooltip>
      ) : queued ? (
        <Tooltip label="It is in the merge queue and merges when its turn comes">
          <Button size="sm" variant="primary" disabled>
            In merge queue
          </Button>
        </Tooltip>
      ) : checks === "passed" ? (
        <Tooltip label="Merges it now, every Check has passed">
          <Button size="sm" variant="primary" pending={pending === "merge"} disabled={busy} onClick={() => onAct("merge")}>
            {turning("merge")}
            Merge
          </Button>
        </Tooltip>
      ) : checks === "pending" ? (
        <Tooltip label={auto ? "It merges when every Check has passed" : "Merges it when every Check has passed"}>
          <Button size="sm" variant="primary" pending={pending === "auto_merge"} disabled={auto || busy} onClick={() => onAct("auto_merge")}>
            {turning("auto_merge")}
            {auto ? "Auto-merge on" : "Enable auto-merge"}
          </Button>
        </Tooltip>
      ) : null}
      <Tooltip label={reviewed ? "A Job on the code review workflow was dispatched against this pull request" : "Dispatches a Job on the code review workflow against this pull request"}>
        <Button size="sm" variant="secondary" pending={pending === "review"} disabled={busy || reviewed} onClick={() => onAct("review")}>
          {turning("review")}
          {reviewed ? "Review dispatched" : "Review"}
        </Button>
      </Tooltip>
    </div>
  );
}
