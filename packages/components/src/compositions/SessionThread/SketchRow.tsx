import { BadgeCheck, PencilRuler, Reply } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { CardFooter } from "../../primitives/Card/Card";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { forgetKept, readKept } from "../../keep";
import { SketchScene } from "../SketchScene/SketchScene";
import type { ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import type { SessionThreadRow } from "./SessionThread";

/** How a sketch put to the person ended: approved, or sent back with his note in `words`. **Proposed, not on the wire.** */
export type SketchSettled = { how: "approved" | "sent_back"; words?: string; at: string };

/** His answer to a sketch put to him, in `answer_waiting`'s words: approved, or sent back with a note. */
export type SketchAnswer = { choice: 0 } | { text: string };

const SETTLED = {
  approved: { said: "Approved", Glyph: BadgeCheck },
  sent_back: { said: "Sent back", Glyph: Reply },
} as const;

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
  const noteKey = `${keep ?? `sketch:${row.id}`}:note`;
  const [noting, setNoting] = useState(() => readKept<string>(noteKey) !== undefined);
  const [note, setNote] = useState("");
  const settled = row.settled;
  useEffect(() => {
    if (settled !== undefined) forgetKept(noteKey);
  }, [settled, noteKey]);
  const open = row.asked === true && settled === undefined && row.replaced !== true && onAnswer !== undefined;
  const state = settled === undefined ? undefined : SETTLED[settled.how];
  const cancel = () => {
    forgetKept(noteKey);
    setNote("");
    setNoting(false);
  };
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
        {!open ? null : (
          <CardFooter role="group" aria-label="Answers">
            {noting ? (
              <>
                <Button size="sm" variant="primary" disabled={note.trim() === ""} onClick={() => onAnswer(row.sketch, { text: note.trim() })}>
                  Send back
                </Button>
                <Button size="sm" variant="ghost" onClick={cancel}>
                  Cancel
                </Button>
              </>
            ) : (
              <>
                <Button size="sm" variant="primary" onClick={() => onAnswer(row.sketch, { choice: 0 })}>
                  Approve
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setNoting(true)}>
                  Send back
                </Button>
              </>
            )}
          </CardFooter>
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
export function SketchReading({ scene, title, against, notes, marked }: { scene: unknown; title?: string; against?: { title: string; scene: unknown }; notes?: ViewerNotesScope; marked?: readonly string[] }) {
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
    </div>
  );
}
