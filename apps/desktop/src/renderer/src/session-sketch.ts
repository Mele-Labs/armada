// A Session's sketch as the scene `SketchScene` draws. The agent's is a scene already; the person's, drawn on the
// pad, is boxes and joins, which a scene says as they are.

import { useMemo } from "react";
import type { Scene, SessionThreadRow } from "@armada/components";
import type { SessionAttachment, SessionSketch, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { sceneOfWire } from "@armada/screens/src/sessions-wire";

import { useRevisionReads } from "./revision-reads";

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
export function readingOfSketch(one: Sketch): { scene: Scene; title: string; against?: { title: string; scene: Scene } } {
  return {
    title: one.title,
    scene: sceneOfSketch(one.drawing),
    ...(one.against === undefined ? {} : { against: { title: one.against.title, scene: sceneOfSketch(one.against.drawing) } }),
  };
}

/**
 * The scenes of each sketch Fleet kept more than one drawing of, by the sketch's id, oldest first. **A sketch is listed once every
 * one of its revisions is read**, so a menu over them never offers one it cannot draw.
 */
export function useSketchRevisions(draft: SessionsDraft | undefined, session: string, attachments: readonly SessionAttachment[]): ReadonlyMap<string, readonly Scene[]> {
  const redrawn = attachments.flatMap((one) => (one.kind === "sketch" && one.revisions !== undefined && one.revisions.length > 1 ? [{ id: one.id, paths: one.revisions }] : []));
  const read = useRevisionReads(draft, session, redrawn.flatMap((one) => one.paths), sceneOfWire);
  const key = redrawn.map((one) => `${one.id}:${String(one.paths.length)}`).join("|");
  return useMemo(() => {
    const out = new Map<string, readonly Scene[]>();
    let at = 0;
    for (const one of redrawn) {
      const scenes = read.slice(at, at + one.paths.length);
      at += one.paths.length;
      if (scenes.every((scene): scene is Scene => scene !== undefined)) out.set(one.id, scenes);
    }
    return out;
  }, [read, key]);
}

/** The thread's rows with each sketch's revisions on its newest card. */
export function withSketchRevisions(rows: SessionThreadRow[], revisions: ReadonlyMap<string, readonly Scene[]>): SessionThreadRow[] {
  if (revisions.size === 0) return rows;
  return rows.map((row) => {
    const scenes = row.kind === "sketch" && row.replaced !== true ? revisions.get(row.sketch) : undefined;
    return scenes === undefined ? row : { ...row, revisions: scenes };
  });
}
