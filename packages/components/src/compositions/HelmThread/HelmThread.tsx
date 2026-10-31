import { useContext, type ReactNode } from "react";
import { Prose, ProseRefs } from "../../primitives/Prose/Prose";
import { titleWithoutReference } from "../SessionComposer/InlineTag";
import type { ComposerTag } from "../SessionComposer/InlineTag";
import { placeTagsIn, SentWith } from "../SessionThread/SessionThread";
import type { ThoughtTarget } from "../ThoughtCard/ThoughtCard";
import { SentTo } from "./SentTo";


/**
 * Helm's own conversation, under the dock's questions — #944.
 *
 * **Two voices only, and neither is `ActivityLog`'s three.** A person and
 * Helm are the whole of this thread; a Job's `armada`/`drone`/`fleet` split
 * answers a different question, about a step nobody typed into.
 */
export type HelmRowActor = "you" | "helm";

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
  /** Where these words went instead of to Helm: the Session, pressed to open it. */
  sent?: { target: ThoughtTarget; onOpen: () => void };
  /** What the person sent with the words: pictures and files, sketches, and the issues, pull requests and Sessions tagged in them. */
  files?: readonly { id: string; name: string; src?: string }[];
  sketches?: readonly { id: string; title: string }[];
  tags?: readonly ComposerTag[];
};

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
    if (tag.kind === "branch" || tag.kind === "report" || refs === null) return;
    refs.open({ kind: tag.kind, id: tag.id, title: titleWithoutReference(tag) });
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
