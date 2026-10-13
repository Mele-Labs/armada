import { useContext, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Prose, ProseRefs } from "../../primitives/Prose/Prose";
import type { ComposerTag } from "../SessionComposer/InlineTag";
import { placeTagsIn, SentWith } from "../SessionThread/SessionThread";
import { THOUGHT_KIND } from "../ThoughtCard/ThoughtCard";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";

/**
 * Helm's own conversation, under the dock's questions — #944.
 *
 * **Two voices only, and neither is `ActivityLog`'s three.** A person and
 * Helm are the whole of this thread; a Job's `armada`/`drone`/`fleet` split
 * answers a different question, about a step nobody typed into.
 */
export type HelmRowActor = "you" | "helm";

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

/** One line of the thread, already worded — `packages/screens` folds the wire into this. */
export type HelmThreadRow = {
  /** Stable across re-renders. The thread's own order. */
  id: string;
  at: string;
  actor: HelmRowActor;
  message: ReactNode;
  /** Machine-derived — a tool call — rather than prose. */
  mono?: boolean;
  /** A trailing caption — the reply's own cost, `$0.02 · 3 turns`. */
  meta?: ReactNode;
  /** An approval Helm asked for, placed within this reply. */
  cards?: HelmApprovalCard[];
  /** Where these words went instead of to Helm: the Session or Job, pressed to open it. */
  sent?: { target: ThoughtTarget; onOpen: () => void };
  /** What the person sent with the words: pictures and files, sketches, and the issues, pull requests and Sessions tagged in them. */
  files?: readonly { id: string; name: string; src?: string }[];
  sketches?: readonly { id: string; title: string }[];
  tags?: readonly ComposerTag[];
};

function ApprovalCard({ jobHandle, workflow, stepCount, state, onApprove, onDismiss }: HelmApprovalCard) {
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

/** One compact line under the owner's words: where they went. Pressing it opens that Session or Job. */
function SentTo({ target, onOpen }: { target: ThoughtTarget; onOpen: () => void }) {
  const { Glyph, said } = THOUGHT_KIND[target.kind];
  return (
    <div className="armada-helm-sent">
      <span className="armada-helm-sent__label">Sent to</span>
      <button type="button" className="armada-helm-sent__chip" aria-label={`Sent to ${said} ${target.title}`} onClick={onOpen}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
        <span className="armada-helm-sent__title">{target.title}</span>
        <span className="armada-helm-sent__repository mono">{target.repository}</span>
      </button>
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

export type HelmThreadProps = {
  rows: HelmThreadRow[];
  /** A reply is on its way. Draws the working line after the last row. */
  replying?: boolean;
  /** What an empty, unstarted thread says. Absent draws nothing: an empty slot stays empty. */
  emptyNote?: ReactNode;
  /** Fleet unreachable, or the socket failed. Drawn above the rows, never in place of them. */
  notice?: ReactNode;
};

export function HelmThread({
  rows,
  replying = false,
  emptyNote,
  notice,
}: HelmThreadProps) {
  const refs = useContext(ProseRefs);
  // A chip in a row of the person's is a link where the host can open what it names.
  const openTag = (tag: ComposerTag): void => {
    if (tag.kind === "branch" || refs === null) return;
    refs.open({ kind: tag.kind, id: tag.id, title: tag.title.replace(/^#\d+\s*/, "") });
  };
  return (
    <div className="armada-helm-thread">
      {notice === undefined ? null : (
        <p className="armada-helm-thread__notice" role="alert">
          {notice}
        </p>
      )}
      {rows.length === 0 && !replying ? (
        emptyNote === undefined ? null : (
          <p className="armada-helm-thread__empty" role="note">
            {emptyNote}
          </p>
        )
      ) : (
        <ol className="armada-helm-thread__rows">
          {rows.map((row) => {
            const placed = row.tags === undefined || row.tags.length === 0 || typeof row.message !== "string" ? undefined : placeTagsIn(row.message, row.tags, refs === null ? undefined : openTag);
            const loose = placed === undefined ? (row.tags ?? []) : placed.loose;
            return (
            <li className="armada-helm-thread__row" key={row.id} data-actor={row.actor}>
              <div className="armada-helm-thread__head">
                <span className="armada-helm-thread__who">{row.actor === "you" ? "You" : "Helm"}</span>
                <span className="armada-helm-thread__at">{row.at}</span>
              </div>
              <div className="armada-helm-thread__message" data-mono={row.mono || undefined}>
                {placed?.parts != null ? (
                  <p className="armada-session-said">{placed.parts}</p>
                ) : typeof row.message === "string"
                  ? row.mono
                    ? row.message
                    : <Prose text={row.message} />
                  : row.message}
              </div>
              {row.cards === undefined
                ? null
                : row.cards.map((card) => <ApprovalCard key={card.id} {...card} />)}
              <SentWith row={row} tags={loose} />
              {row.sent === undefined ? null : <SentTo {...row.sent} />}
              {row.meta === undefined ? null : <p className="armada-helm-thread__meta">{row.meta}</p>}
            </li>
            );
          })}
        </ol>
      )}
      {!replying ? null : (
        <p className="armada-helm-thread__replying" role="status">
          <span className="armada-helm-thread__working" aria-hidden />
          Helm is replying…
        </p>
      )}
    </div>
  );
}
