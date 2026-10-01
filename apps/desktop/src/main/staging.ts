// Where main writes a picture for Fleet to copy into its own keeping — a capture window's frame,
// and a picture pasted onto a Studio. **Main writes every staged file**, so the only path Fleet is
// handed is one main chose.

import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { app, nativeImage } from "electron";
import type { StagedFrame } from "@armada/protocol";

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
