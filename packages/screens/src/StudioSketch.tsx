// A Studio's Sketch, open on the pad — the owner's call of 1 Oct 2026: a
// Studio's Sketch and the dispatch composer's pad are one drawing. Its own file
// because `Studios.tsx` is the board and this is one layer over it.
//
// **Saved when the pad closes**, once, as the whole drawing. Nothing is written
// while a person is still drawing, so a Studio never holds half a stroke, and a
// refusal is said on the pad, which stays open so nothing drawn is lost. The
// decision of 1 Oct 2026, *a Sketch is the pad*, chose this save point.

import { useState } from "react";
import type { ReactNode } from "react";

import { Alert, Sheet, SketchPad, STUDIO_NODE_KIND } from "@armada/components";
import type { Outcome, SketchToKeep, StudioNode, StudioPosition } from "@armada/protocol";

import { said } from "./copy";
import {
  NOTHING_DRAWN,
  isDrawn,
  nextPictureId,
  nextShapeId,
  withBody,
  withJoin,
  withPicture,
  withPlace,
  withShape,
  withStroke,
  withoutLastStroke,
  withoutShapes,
} from "./draft/sketch";
import type { Drawing } from "./draft/sketch";
import { drawnAs, keptOf, padOf, toKeep } from "./studio-sketch";

/** What the pad is, read to somebody who cannot see it. */
const PAD_LABEL = "The sketch on this Studio";

/** Said over the pad while a save is out to Fleet. */
const SAVING = "Keeping the sketch on this Studio.";

/**
 * One pad open: a Sketch already on the Studio, or one being placed. `kept` is
 * the pictures Fleet already holds, by id; `opened` is what it was drawn as
 * when it opened, so closing it unchanged writes nothing.
 */
type Open = {
  nodeId: string | null;
  position: StudioPosition;
  drawing: Drawing;
  kept: ReadonlySet<string>;
  opened: string;
};

export type StudioSketchProps = {
  /** Whether the pad may be drawn on: an editable Studio on a live connection. */
  editable: boolean;
  /** A kept picture's `blob:`, once the board has read it, or `undefined` before. */
  srcOf: (nodeId: string, pictureId: string) => string | undefined;
  /** Put a new Sketch on the Studio, where it was placed. */
  onAdd: (drawing: SketchToKeep, position: StudioPosition) => Promise<Outcome>;
  /** Keep a Sketch's drawing as it was left. */
  onSave: (nodeId: string, drawing: SketchToKeep) => Promise<Outcome>;
};

export type StudioSketch = {
  /** Open the pad blank, for a Sketch placed at `position`. */
  placed: (position: StudioPosition) => void;
  /** Open the pad on a Sketch already on the Studio. */
  opened: (node: Extract<StudioNode, { kind: "sketch" }>) => void;
  /** The layer, or `null` with no pad open. */
  sheet: ReactNode;
};

/** A `blob:` read back into bytes, for main to stage. */
async function bytesOf(src: string): Promise<Uint8Array> {
  return new Uint8Array(await (await fetch(src)).arrayBuffer());
}

export function useStudioSketch({ editable, srcOf, onAdd, onSave }: StudioSketchProps): StudioSketch {
  const [open, setOpen] = useState<Open | null>(null);
  const [saving, setSaving] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);

  function start(next: Open): void {
    setRefused(null);
    setSaving(false);
    setOpen(next);
  }

  function draw(change: (drawing: Drawing) => Drawing): void {
    setOpen((held) => (held === null ? held : { ...held, drawing: change(held.drawing) }));
  }

  /**
   * Close, keeping what was drawn. **Unchanged, or read-only, writes
   * nothing**, and a pad placed and left blank is no Sketch at all.
   */
  function close(): void {
    if (open === null || saving) return;
    const changed = drawnAs(open.drawing) !== open.opened;
    if (!editable || !changed || (open.nodeId === null && !isDrawn(open.drawing))) {
      setOpen(null);
      return;
    }
    setSaving(true);
    setRefused(null);
    const { nodeId, position, drawing, kept } = open;
    void toKeep(drawing, kept, bytesOf)
      .then((keeping) => (nodeId === null ? onAdd(keeping, position) : onSave(nodeId, keeping)))
      .then((outcome) => {
        setSaving(false);
        if (outcome.ok) setOpen(null);
        else setRefused(outcome.why === "refused" ? outcome.error.message : said(outcome));
      });
  }

  const shown =
    open === null
      ? null
      : open.drawing.pictures.map((one) =>
          open.nodeId !== null && open.kept.has(one.id)
            ? { ...one, src: srcOf(open.nodeId, one.id) ?? "" }
            : one,
        );

  const sheet =
    open === null || shown === null ? null : (
      <Sheet
        open
        contained
        size="widest"
        title={STUDIO_NODE_KIND.sketch}
        {...(saving ? { subtitle: SAVING } : {})}
        onClose={close}
        bands={
          refused === null ? null : (
            <Alert tone="escalated" title="Fleet did not take that">
              {refused}
            </Alert>
          )
        }
      >
        <SketchPad
          label={PAD_LABEL}
          boxes={open.drawing.shapes}
          lines={open.drawing.joins}
          strokes={open.drawing.strokes}
          pictures={shown}
          onAdd={(at, body) => draw((one) => withShape(one, { id: nextShapeId(one), x: at.x, y: at.y, body }))}
          onPicture={(picture, at) =>
            draw((one) => withPicture(one, { id: nextPictureId(one), x: at.x, y: at.y, ...picture }))
          }
          onBody={(id, body) => draw((one) => withBody(one, id, body))}
          onMove={(id, at) => draw((one) => withPlace(one, id, at))}
          onRemove={(ids) =>
            // A kept picture taken off is no longer kept here, so a paste
            // that reuses its id is a new picture and sends its bytes.
            setOpen((held) =>
              held === null
                ? held
                : {
                    ...held,
                    drawing: withoutShapes(held.drawing, ids),
                    kept: new Set([...held.kept].filter((id) => !ids.includes(id))),
                  },
            )
          }
          onJoin={(from, to) => draw((one) => withJoin(one, from, to))}
          onDraw={(points) => draw((one) => withStroke(one, points))}
          onUndo={() => draw(withoutLastStroke)}
          disabled={!editable || saving}
        />
      </Sheet>
    );

  return {
    placed: (position) =>
      start({ nodeId: null, position, drawing: NOTHING_DRAWN, kept: new Set(), opened: drawnAs(NOTHING_DRAWN) }),
    opened: (node) => {
      const drawing = padOf(node.drawing);
      start({ nodeId: node.id, position: node.position, drawing, kept: keptOf(node.drawing), opened: drawnAs(drawing) });
    },
    sheet,
  };
}
