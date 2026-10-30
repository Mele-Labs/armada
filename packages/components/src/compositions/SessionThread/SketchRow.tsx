import { BadgeCheck, BellRing, ChevronDown, ChevronRight, Maximize2, PencilRuler, Reply } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Card, CardFooter } from "../../primitives/Card/Card";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept, readKept } from "../../keep";
import { SketchScene } from "../SketchScene/SketchScene";
import type { Scene } from "../SketchScene/scene";
import type { ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import type { SessionThreadRow } from "./SessionThread";
import type { SketchAnswer, SketchSettled } from "./sketch-answer";

export type { SketchAnswer, SketchSettled } from "./sketch-answer";

const SETTLED = {
  approved: { said: "Approved", Glyph: BadgeCheck },
  sent_back: { said: "Sent back", Glyph: Reply },
} as const;

/**
 * What answers a sketch or a document put to the person, wherever it is read: the note he writes to send it back, how it
 * ended once it has, and Approve with Send back while it waits. One part for the thread card, the panel and the Needs you
 * row, so they answer it the same way and a note written in one is kept under `keep`. `id` is handed back with the answer.
 */
export function ApprovalAnswers({ id, keep, settled, open, onAnswer }: { id: string; keep: string; settled?: SketchSettled; open: boolean; onAnswer?: (id: string, answer: SketchAnswer) => void }) {
  const noteKey = `${keep}:note`;
  const [noting, setNoting] = useState(() => readKept<string>(noteKey) !== undefined);
  const [note, setNote] = useState("");
  useEffect(() => {
    if (settled !== undefined) forgetKept(noteKey);
  }, [settled, noteKey]);
  const state = settled === undefined ? undefined : SETTLED[settled.how];
  const cancel = () => {
    forgetKept(noteKey);
    setNote("");
    setNoting(false);
  };
  return (
    <>
      {open && noting ? <Textarea aria-label="Note" rows={3} autoFocus keep={noteKey} value={note} onChange={(event) => setNote(event.currentTarget.value)} /> : null}
      {settled?.how === "sent_back" && settled.words !== undefined ? <blockquote className="armada-session-sketch__note">{settled.words}</blockquote> : null}
      {state === undefined ? null : (
        <p className="armada-session-sketch__settled" data-how={settled!.how}>
          <Tooltip label={state.said}>
            <span className="armada-session-sketch__mark" role="img" aria-label={state.said}>
              <state.Glyph size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span aria-hidden>{state.said}</span>
        </p>
      )}
      {!open || onAnswer === undefined ? null : (
        <CardFooter role="group" aria-label="Answers">
          {noting ? (
            <>
              <Button size="sm" variant="primary" disabled={note.trim() === ""} onClick={() => onAnswer(id, { text: note.trim() })}>
                Send back
              </Button>
              <Button size="sm" variant="ghost" onClick={cancel}>
                Cancel
              </Button>
            </>
          ) : (
            <>
              <Button size="sm" variant="primary" onClick={() => onAnswer(id, { choice: 0 })}>
                Approve
              </Button>
              <Button size="sm" variant="secondary" onClick={() => setNoting(true)}>
                Send back
              </Button>
            </>
          )}
        </CardFooter>
      )}
    </>
  );
}

/**
 * A sketch the agent drew, where in the thread it did: a header band over the drawing itself, shrunk (`SketchScene`'s
 * `preview`, so the frames, icons, colours and edges are the panel's), and a
 * press that opens the sketch in the panel. It plays the sketch's steps while it is pointed at or focused. **A card whose sketch was replaced is dashed and stands
 * back**, because its press opens the sketch that took its place. A sketch put to the person (`asked`) is answered under
 * it as a draft card is: Approve, and Send back with a note, then how it ended. `keep` names it for the note he types.
 */
export function SketchRow({ row, keep, onOpen, onAnswer }: { row: Extract<SessionThreadRow, { kind: "sketch" }>; keep?: string; onOpen?: (sketch: string) => void; onAnswer?: (sketch: string, answer: SketchAnswer) => void }) {
  // The steps loop while the card is pointed at or focused, and the card is a still drawing the rest of the time.
  const [pointed, setPointed] = useState(false);
  const [focused, setFocused] = useState(false);
  const settled = row.settled;
  const open = row.asked === true && settled === undefined && row.replaced !== true && onAnswer !== undefined;
  const press = (
    <button
      type="button"
      className="armada-session-sketch__press"
      aria-label={`Open sketch ${row.title}`}
      onClick={() => onOpen?.(row.sketch)}
      onMouseEnter={() => setPointed(true)}
      onMouseLeave={() => setPointed(false)}
      onFocus={() => setFocused(true)}
      onBlur={() => setFocused(false)}
    >
      <span className="armada-session-sketch__band">
        <PencilRuler size={12} strokeWidth={2} aria-hidden />
        <span className="armada-session-sketch__title">{row.title}</span>
        {row.replaced === true ? <span className="armada-session-sketch__state">Replaced</span> : null}
        {row.against === undefined ? null : <span className="armada-session-sketch__state">Against {row.against}</span>}
        <Stamp still at={row.at} className="armada-session-sketch__at" />
      </span>
      <span className="armada-session-sketch__glass">
        <SketchScene scene={row.drawing} {...(row.againstScene === undefined ? {} : { against: row.againstScene })} preview playing={pointed || focused} />
      </span>
    </button>
  );
  if (row.asked !== true && settled === undefined) {
    return (
      <li className="armada-session-sketch" data-replaced={row.replaced || undefined}>
        {press}
      </li>
    );
  }
  return (
    <li className="armada-session-sketch" data-replaced={row.replaced || undefined} data-asked>
      <div className="armada-session-sketch__asked" role="article" aria-label={`Sketch: ${row.title}`} data-settled={settled?.how}>
        {press}
        {!open && settled === undefined ? null : (
          <div className="armada-session-sketch__answers">
            <ApprovalAnswers id={row.sketch} keep={keep ?? `sketch:${row.id}`} {...(settled === undefined ? {} : { settled })} open={open} {...(onAnswer === undefined ? {} : { onAnswer })} />
          </div>
        )}
      </div>
    </li>
  );
}

/**
 * A sketch read in the panel, read-only: pan, zoom and the steps, drawn by `SketchScene`. **Given
 * the sketch it was drawn against, it is read as a change to it** (added green, removed red,
 * changed amber), and a band offers the whole drawing instead.
 */
export function SketchReading({ scene, title, against, notes, marked, answer }: { scene: unknown; title?: string; against?: { title: string; scene: unknown }; notes?: ViewerNotesScope; marked?: readonly string[]; /** Present while the sketch waits on him: the same Approve and Send back the thread card has. */ answer?: { sketch: string; keep: string; onAnswer: (sketch: string, answer: SketchAnswer) => void } }) {
  const [changes, setChanges] = useState(true);
  return (
    <div className="armada-session-sketch-read">
      {against === undefined ? null : (
        <div className="armada-session-sketch-read__band">
          <span className="armada-session-sketch-read__against">Against {against.title}</span>
          <span className="armada-session-sketch-read__reads" role="group" aria-label="Read as">
            <Button size="sm" variant={changes ? "secondary" : "ghost"} aria-pressed={changes} onClick={() => setChanges(true)}>
              Changes
            </Button>
            <Button size="sm" variant={changes ? "ghost" : "secondary"} aria-pressed={!changes} onClick={() => setChanges(false)}>
              Whole
            </Button>
          </span>
        </div>
      )}
      <SketchScene scene={scene} autoplay {...(title === undefined ? {} : { title })} {...(against === undefined || !changes ? {} : { against: against.scene })} {...(notes === undefined ? {} : { notes })} {...(marked === undefined ? {} : { marked })} />
      {answer === undefined ? null : (
        <div className="armada-session-sketch__answers">
          <ApprovalAnswers id={answer.sketch} keep={answer.keep} open onAnswer={answer.onAnswer} />
        </div>
      )}
    </div>
  );
}

/** A sketch waiting on the person, pinned as the plan to approve is. */
export type SketchPinProps = { sketch: string; title: string; scene: Scene; against?: { title: string; scene: Scene }; /** Where his notes on it are held, and the key the note he types is kept under. */ notes?: ViewerNotesScope; keep: string; onAnswer: (sketch: string, answer: SketchAnswer) => void; /** Opens it in the side panel, bigger, as the thread card's press does. */ onOpen?: (sketch: string) => void };

/**
 * A sketch put to the person, as the thread's last card where the plan to approve stands: the drawing with the canvas's own pan, zoom
 * and fit and his notes, and under it Approve and Send back, which are `SketchRow`'s. It is the thread's, so it goes when he answers.
 * **A chevron folds it to its header line**, the glyph, the title and the bell with the keys at its end, as the plan folds to its band.
 * Folded, Send back opens the note under that line, and the drawing stays folded.
 */
export function SketchPin({ sketch, title, scene, against, notes, keep, onAnswer, onOpen }: SketchPinProps) {
  const [folded, setFolded] = useState(false);
  const Fold = folded ? ChevronRight : ChevronDown;
  const said = folded ? "Unfold" : "Fold";
  return (
    <Card flat className="armada-session-thread__ask armada-session-sketch-pin" role="article" aria-label={`Sketch to approve: ${title}`} data-folded={folded || undefined}>
      <div className="armada-session-thread__plan-head">
        <span className="armada-session-sketch-pin__band">
          <PencilRuler size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-thread__eyebrow">{title}</span>
          <Tooltip label="Sketch waiting for approval">
            <span className="armada-session-thread__settled" role="img" aria-label="Sketch waiting for approval" data-state="waiting">
              <BellRing size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span className="armada-session-sketch-pin__tools">
            {onOpen === undefined ? null : (
              <Tooltip label="Open in the panel">
                <Button className="armada-session-sketch-pin__open" variant="ghost" size="sm" iconOnly aria-label="Open in the panel" onClick={() => onOpen(sketch)}>
                  <Maximize2 size={12} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
            )}
            <Tooltip label={said}>
              <Button className="armada-session-sketch-pin__fold" variant="ghost" size="sm" iconOnly aria-label={said} aria-expanded={!folded} onClick={() => setFolded(!folded)}>
                <Fold size={12} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </span>
        </span>
      </div>
      {folded ? null : (
        <div className="armada-session-sketch-pin__read">
          <SketchReading scene={scene} title={title} {...(against === undefined ? {} : { against })} {...(notes === undefined ? {} : { notes })} />
        </div>
      )}
      <div className="armada-session-sketch__answers">
        <ApprovalAnswers id={sketch} keep={keep} open onAnswer={onAnswer} />
      </div>
    </Card>
  );
}
