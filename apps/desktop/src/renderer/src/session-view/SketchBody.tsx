// A sketch read as the reading sheet reads it, with no sheet round it: the drawing, its revisions, and what a note on it names marked. The reading
// sheet over a Job's ledger and the Job's review sheet both draw it, so a sketch reads one way wherever it is opened. With `answers` it carries the
// thread card's Approve and Send back while the sketch waits on him; the review sheet holds those in its own head and passes none.

import { useState } from "react";
import { RevisionMenu, SketchPreview, SketchReading, sketchAtRevision } from "@armada/components";
import type { SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";

import { dismissWaiting } from "../cockpit/dismissed";
import { readingOfSketch, sceneOfSketch, useSketchRevisions } from "../session-sketch";
import { sketchAnswerOf } from "../session-thread-rows";

export type SketchAttachment = Extract<
  SessionAttachment,
  {
    kind: "sketch";
  }
>;

/** Whether a sketch is drawn as a scene: the agent's is, and so is any with nodes. The person's pad drawing is boxes and lines. */
export const drawnAsScene = (one: SketchAttachment): boolean =>
  one.by === "agent" || "nodes" in one.drawing;

export function SketchBody({
  sessionId,
  one,
  draft,
  marked,
  answers = false,
  onRevising,
}: {
  sessionId: string;
  one: SketchAttachment;
  draft: SessionsDraft | undefined;
  /** What a sent note named in it, marked as it opens: a sketch's parts. */
  marked?: readonly string[];
  /** Offer Approve and Send back where the sketch waits on him. */
  answers?: boolean;
  /** The revision menu opened or closed, for a sheet whose Esc closes the menu first. */
  onRevising?: (open: boolean) => void;
}) {
  const answer =
    answers &&
    one.asked === true &&
    one.withdrawn !== true &&
    one.settled === undefined &&
    draft?.answerSketch !== undefined
      ? {
          sketch: one.id,
          keep: `sketch:${sessionId}:${one.id}`,
          onAnswer: (sketch: string, given: Parameters<typeof sketchAnswerOf>[2]) =>
            void draft.answerSketch?.(
              sketchAnswerOf(
                {
                  id: sessionId,
                  attachments: [
                    one,
                  ],
                },
                sketch,
                given,
              ),
            ),
          onWithdraw: (sketch: string) => void dismissWaiting(sessionId, sketch),
        }
      : undefined;
  // A sketch with revisions offers each drawing, as the thread card does, at the end of the reading's band.
  const revisions = useSketchRevisions(draft, sessionId, [
    one,
  ]).get(one.id);
  const [picked, setPicked] = useState<number | undefined>(undefined);
  const [revising, setRevising] = useState(false);
  const reading = readingOfSketch(one);
  const shown = sketchAtRevision(revisions, picked, reading);
  const revisionMenu =
    revisions === undefined || shown?.revision === undefined ? undefined : (
      <RevisionMenu
        count={revisions.length}
        revision={shown.revision}
        align="end"
        open={revising}
        onOpenChange={(open) => {
          setRevising(open);
          onRevising?.(open);
        }}
        onPick={(revision) => setPicked(revision === revisions.length ? undefined : revision)}
      />
    );
  if (one.by !== "agent" && !("nodes" in one.drawing))
    return (
      <SketchPreview
        label={one.title}
        boxes={one.drawing.boxes}
        lines={one.drawing.lines}
        strokes={one.drawing.strokes ?? []}
        pictures={[]}
      />
    );
  return (
    <SketchReading
      key={shown?.revision}
      scene={shown?.scene ?? sceneOfSketch(one.drawing)}
      title={one.title}
      {...(shown?.against === undefined
        ? {}
        : {
            against: shown.against,
          })}
      {...(shown?.earlier === true
        ? {}
        : {
            notes: {
              key: `sketch:${sessionId}:${one.id}`,
              job: sessionId,
            },
          })}
      {...(marked === undefined || shown?.earlier === true
        ? {}
        : {
            marked,
          })}
      {...(answer === undefined
        ? {}
        : {
            answer,
          })}
      {...(revisionMenu === undefined
        ? {}
        : {
            menu: revisionMenu,
          })}
    />
  );
}
