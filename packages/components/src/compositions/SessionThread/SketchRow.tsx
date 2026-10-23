import { PencilRuler } from "lucide-react";
import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { Stamp } from "../../primitives/Stamp/Stamp";
import { SketchScene } from "../SketchScene/SketchScene";
import type { ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import type { SessionThreadRow } from "./SessionThread";

/**
 * A sketch the agent drew, where in the thread it did: a header band over the drawing itself, shrunk (`SketchScene`'s
 * `preview`, so the frames, icons, colours and edges are the panel's), and a
 * press that opens the sketch in the panel. It plays the sketch's steps while it is pointed at or focused. **A card whose sketch was replaced is dashed and stands
 * back**, because its press opens the sketch that took its place.
 */
export function SketchRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "sketch" }>; onOpen?: (sketch: string) => void }) {
  // The steps loop while the card is pointed at or focused, and the card is a still drawing the rest of the time.
  const [pointed, setPointed] = useState(false);
  const [focused, setFocused] = useState(false);
  return (
    <li className="armada-session-sketch" data-replaced={row.replaced || undefined}>
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
