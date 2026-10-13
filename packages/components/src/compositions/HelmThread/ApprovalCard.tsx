import { Button } from "../../primitives/Button/Button";

/**
 * What one `ask_person_to_approve` call draws as, within Helm's reply.
 * `#1041`.
 *
 * **Never the model's own words.** `jobHandle`, `workflow` and `stepCount`
 * come from `BridgeState.jobs` at render time, resolved by whoever builds
 * this prop — never from anything Helm's tool call said, and never frozen at
 * the moment Helm asked: a card re-renders `elsewhere` the instant the Board
 * says the Job moved by any other road. `unknown` is the one exception: a
 * Job id the caller could not resolve at all, drawn in words rather than a
 * card missing the facts a card promises.
 */
export type HelmApprovalCardState = "ready" | "pending" | "approved" | "dismissed" | "elsewhere" | "unknown";

export type HelmApprovalCard = {
  /** Stable across re-renders — the call's own id. */
  id: string;
  /** The raw Job id, where the caller could not resolve a handle for it. */
  jobHandle: string;
  /** Absent where the workflow is not known — drawn without it rather than blocking the card. */
  workflow?: string;
  stepCount?: number;
  /**
   * `"pending"` is Approve pressed and Fleet not yet answered — read off the
   * Job leaving `awaiting_approval`, since the press itself is fire-and-forget.
   * The card does not claim `"approved"` before that lands. #1117.
   */
  state: HelmApprovalCardState;
  /** Present where `state` is `"ready"` or `"pending"` — pending keeps Approve's own handler bound, though `Button`'s own pending guard is what actually refuses the second press. */
  onApprove?: () => void;
  onDismiss?: () => void;
};

export function ApprovalCard({ jobHandle, workflow, stepCount, state, onApprove, onDismiss }: HelmApprovalCard) {
  const named = [jobHandle, workflow, stepCount === undefined ? undefined : plural(stepCount)]
    .filter((part): part is string => part !== undefined)
    .join(" · ");
  const pending = state === "pending";
  return (
    <div className="armada-helm-approval-card" data-state={state}>
      <p className="armada-helm-approval-card__job">{named}</p>
      {state === "ready" || pending ? (
        <div className="armada-helm-approval-card__actions">
          <Button variant="primary" size="sm" ground="sunken" pending={pending} onClick={onApprove}>
            {pending ? "Approving…" : "Approve"}
          </Button>
          <Button variant="secondary" size="sm" ground="sunken" disabled={pending} onClick={onDismiss}>
            Not now
          </Button>
        </div>
      ) : (
        <p className="armada-helm-approval-card__state">{STATE_SAID[state]}</p>
      )}
    </div>
  );
}

const STATE_SAID: Record<Exclude<HelmApprovalCardState, "ready" | "pending">, string> = {
  approved: "Approved.",
  dismissed: "Dismissed.",
  elsewhere: "No longer awaiting approval.",
  unknown: "There is no Job with that id.",
};

function plural(stepCount: number): string {
  return `${stepCount} ${stepCount === 1 ? "step" : "steps"}`;
}
