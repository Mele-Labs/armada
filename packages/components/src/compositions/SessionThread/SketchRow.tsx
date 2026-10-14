import { PencilRuler } from "lucide-react";
import { useState } from "react";

import { Button } from "../../primitives/Button/Button";
import { SketchPreview } from "../SketchPreview/SketchPreview";
import { SketchScene } from "../SketchScene/SketchScene";
import { parseScene } from "../SketchScene/scene";
import type { Scene } from "../SketchScene/scene";
import type { SessionThreadRow } from "./SessionThread";

/** The parts of a scene a small preview can stand for: a frame and its words. Grounds and lanes are left out. */
function previewOf(scene: Scene) {
  const parts = scene.nodes.filter((one) => one.kind !== "group" && one.kind !== "lane");
  const kept = new Set(parts.map((one) => one.id));
  return {
    boxes: parts.map((one) => ({ id: one.id, x: one.x, y: one.y, body: one.title ?? one.body?.split("\n")[0] ?? "" })),
    lines: scene.edges.filter((one) => kept.has(one.from) && kept.has(one.to)).map((one) => ({ id: one.id, from: one.from, to: one.to })),
  };
}

/**
 * A sketch the agent drew, where in the thread it did: a header band over a small drawing, and a
 * press that opens the sketch in the panel. **A card whose sketch was replaced is dashed and stands
 * back**, because its press opens the sketch that took its place.
 */
export function SketchRow({ row, onOpen }: { row: Extract<SessionThreadRow, { kind: "sketch" }>; onOpen?: (sketch: string) => void }) {
  const scene = parseScene(row.drawing);
  const small = scene === undefined ? { boxes: [], lines: [] } : previewOf(scene);
  return (
    <li className="armada-session-sketch" data-replaced={row.replaced || undefined}>
      <button type="button" className="armada-session-sketch__press" aria-label={`Open sketch ${row.title}`} onClick={() => onOpen?.(row.sketch)}>
        <span className="armada-session-sketch__band">
          <PencilRuler size={12} strokeWidth={2} aria-hidden />
          <span className="armada-session-sketch__title">{row.title}</span>
          {row.replaced === true ? <span className="armada-session-sketch__state">Replaced</span> : null}
          {row.against === undefined ? null : <span className="armada-session-sketch__state">Against {row.against}</span>}
          <span className="armada-session-sketch__at">{row.at}</span>
        </span>
        <span className="armada-session-sketch__glass">
          <SketchPreview label={row.title} boxes={small.boxes} lines={small.lines} strokes={[]} pictures={[]} />
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
export function SketchReading({ scene, against }: { scene: unknown; against?: { title: string; scene: unknown } }) {
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
      <SketchScene scene={scene} {...(against === undefined || !changes ? {} : { against: against.scene })} />
    </div>
  );
}
