// Where main writes a picture for Fleet to copy into its own keeping — a capture window's frame,
// and a picture pasted onto a Studio. **Main writes every staged file**, so the only path Fleet is
// handed is one main chose.

import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { app, nativeImage } from "electron";
import type { SketchDrawn, StagedFrame } from "@armada/protocol";

/** A PNG written where a frame is staged. */
export async function stagePng(png: Buffer, width: number, height: number): Promise<StagedFrame> {
  const dir = join(app.getPath("temp"), "armada-frames", randomUUID());
  await mkdir(dir, { recursive: true });
  const staged = join(dir, "frame.png");
  await writeFile(staged, png);
  return { staged_path: staged, width, height };
}

/**
 * Pasted bytes, staged as a PNG at the size they decode to. `null` where they are not an image —
 * Chromium hands a pasted picture over as PNG, so that is a renderer sending something else.
 * One over Fleet's bound is still staged: Fleet refuses it, and the refusal says why.
 */
export async function stagedPicture(bytes: Uint8Array): Promise<StagedFrame | null> {
  const image = nativeImage.createFromBuffer(Buffer.from(bytes));
  if (image.isEmpty()) return null;
  const { width, height } = image.getSize();
  return stagePng(image.toPNG(), width, height);
}

/** Whether a value is text, a whole number, or a list of what `each` accepts. */
const isText = (value: unknown): value is string => typeof value === "string";
const isWhole = (value: unknown): value is number => Number.isInteger(value);
const listOf = (value: unknown, each: (item: Record<string, unknown>) => boolean): boolean =>
  Array.isArray(value) && value.every((item) => typeof item === "object" && item !== null && each(item as Record<string, unknown>));

/**
 * A drawing the renderer handed over, with each new picture's bytes staged —
 * 1 Oct 2026. `null` where it is not a drawing's shape, or a picture's bytes
 * are not an image. **Only the shape is checked here**: whether the drawing
 * holds together is Fleet's to refuse, by the domain's own rule.
 */
export async function stagedSketch(value: unknown): Promise<SketchDrawn | null> {
  const drawing = (value ?? {}) as Record<string, unknown>;
  const placed = (item: Record<string, unknown>) => isText(item.id) && isWhole(item.x) && isWhole(item.y);
  const shaped =
    listOf(drawing.boxes, (box) => placed(box) && isText(box.body)) &&
    listOf(drawing.joins, (join) => isText(join.id) && isText(join.from) && isText(join.to)) &&
    listOf(drawing.strokes, (stroke) => isText(stroke.id) && listOf(stroke.points, (at) => isWhole(at.x) && isWhole(at.y))) &&
    listOf(
      drawing.pictures,
      (picture) =>
        placed(picture) &&
        isWhole(picture.width) &&
        isWhole(picture.height) &&
        (picture.bytes === undefined || picture.bytes instanceof Uint8Array),
    );
  if (!shaped) return null;
  const pictures = drawing.pictures as { id: string; x: number; y: number; width: number; height: number; bytes?: Uint8Array }[];
  const drawn: SketchDrawn["pictures"] = [];
  for (const { id, x, y, width, height, bytes } of pictures) {
    if (bytes === undefined) {
      drawn.push({ id, x, y, width, height });
      continue;
    }
    const staged = await stagedPicture(bytes);
    if (staged === null) return null;
    drawn.push({ id, x, y, width, height, staged });
  }
  return {
    boxes: drawing.boxes as SketchDrawn["boxes"],
    joins: drawing.joins as SketchDrawn["joins"],
    strokes: drawing.strokes as SketchDrawn["strokes"],
    pictures: drawn,
  };
}
