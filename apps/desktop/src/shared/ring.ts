// Where a note's picture is ringed. A note sends the whole window, and the box the owner picked is ringed on it and tagged
// with the note's number in the turn, so "3." in the message is the picture tagged 3. `main/capture/burn.ts` draws it.

import type { Box } from "./annotations";

/** A box on a picture, in the picture's pixels, and its pixels per CSS pixel, which the ring's stroke is drawn to. */
export type Ring = { x: number; y: number; width: number; height: number; scale: number };

/** A PNG's size, read off its header; null for bytes that are not one. */
export function pngSize(bytes: Uint8Array): { width: number; height: number } | null {
  // The signature, then IHDR's length and name: the width and height are the eight bytes after them.
  if (bytes.length < 24 || bytes[12] !== 0x49 || bytes[13] !== 0x48 || bytes[14] !== 0x44 || bytes[15] !== 0x52) return null;
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return { width: view.getUint32(16), height: view.getUint32(20) };
}

/** `box`, in the CSS pixels of a viewport `viewportWidth` wide, on a picture of that viewport `pictureWidth` pixels wide. */
export function ringOn(box: Box, viewportWidth: number, pictureWidth: number): Ring {
  const scale = viewportWidth > 0 && pictureWidth > 0 ? pictureWidth / viewportWidth : 1;
  return { x: Math.round(box.x * scale), y: Math.round(box.y * scale), width: Math.round(box.width * scale), height: Math.round(box.height * scale), scale };
}

export function isRing(value: unknown): value is Ring {
  if (typeof value !== "object" || value === null) return false;
  const one = value as Record<string, unknown>;
  return ["x", "y", "width", "height", "scale"].every((key) => typeof one[key] === "number" && Number.isFinite(one[key]));
}
