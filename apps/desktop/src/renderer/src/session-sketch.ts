// A Session's sketch as the scene `SketchScene` draws. The agent's is a scene already; the person's, drawn on the
// pad, is boxes and joins, which a scene says as they are.

import type { Scene } from "@armada/components";
import type { SessionAttachment, SessionSketch } from "@armada/screens/src/draft/sessions";

type Sketch = Extract<SessionAttachment, { kind: "sketch" }>;

export function sceneOfSketch(drawing: SessionSketch | Scene): Scene {
  if ("nodes" in drawing) return drawing;
  return {
    nodes: drawing.boxes.map((one) => ({ id: one.id, kind: "box", x: one.x, y: one.y, body: one.body })),
    edges: drawing.lines.map((one) => ({ id: one.id, from: one.from, to: one.to })),
    ...(drawing.strokes === undefined ? {} : { strokes: drawing.strokes }),
  };
}

/** What the panel reads of a ledger sketch: its scene, and the one it was drawn against. */
export function readingOfSketch(one: Sketch): { scene: Scene; against?: { title: string; scene: Scene } } {
  return {
    scene: sceneOfSketch(one.drawing),
    ...(one.against === undefined ? {} : { against: { title: one.against.title, scene: sceneOfSketch(one.against.drawing) } }),
  };
}
