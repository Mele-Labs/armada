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
 */
export type PullRequestActsProps = {
  state: "draft" | "open" | "merged";
  checks: "pending" | "passed" | "failed";
  /** Auto-merge has been asked for. */
  auto: boolean;
  /** It is in the merge queue: the forge has it, so no act asks for that again. */
  queued?: boolean;
  onAct: (act: "ready" | "merge" | "auto_merge" | "review") => void;
};

export function PullRequestActs({ state, checks, auto, queued = false, onAct }: PullRequestActsProps) {
  if (state === "merged") return null;
  return (
    <div className="armada-pr-acts" role="group" aria-label="Pull request acts">
      {state === "draft" ? (
        <Tooltip label="Marks the pull request ready for review">
          <Button size="sm" variant="primary" onClick={() => onAct("ready")}>
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
          <Button size="sm" variant="primary" onClick={() => onAct("merge")}>
            Merge
          </Button>
        </Tooltip>
      ) : checks === "pending" ? (
        <Tooltip label={auto ? "It merges when every Check has passed" : "Merges it when every Check has passed"}>
          <Button size="sm" variant="primary" disabled={auto} onClick={() => onAct("auto_merge")}>
            {auto ? "Auto-merge on" : "Enable auto-merge"}
          </Button>
        </Tooltip>
      ) : null}
      <Tooltip label="Dispatches a Job on the code review workflow against this pull request">
        <Button size="sm" variant="secondary" onClick={() => onAct("review")}>
          Review
        </Button>
      </Tooltip>
    </div>
  );
}
