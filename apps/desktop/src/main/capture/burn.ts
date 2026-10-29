// The ring and number burned into a note's picture as it is sent, as the held notes panel draws them over it
// (`walk-panel/held-notes.css`): the box outset by --space-1, a 2px --accent stroke at --radius-sm, and the note's number in
// an --accent pill at the ring's top-left corner. Every path that sends a note's picture burns it here.
//
// **Drawn into the bitmap, with digits of its own.** Main has no canvas, and a hidden window to draw one in is a window
// that can reach the owner's screen. Digits are the only text, so a 5x7 face scaled to whole device pixels draws them
// sharp, and a test reads the pixels back without Electron.

import { nativeImage } from "electron";

import tokens from "@armada/tokens/tokens.json";

import type { Ring } from "../../shared/ring";

/** A token's value, as `tokens.json` holds it: the dark theme's, since main is not told which theme the renderer drew. */
const token = (name: string): string => tokens.tokens.find((one) => one.name === name)?.value ?? "";
const px = (name: string): number => Number.parseInt(token(name), 10) || 0;

/** `#RRGGBB` as the bytes a BGRA bitmap holds, opaque. */
function bgra(hex: string): [number, number, number, number] {
  const value = Number.parseInt(hex.replace("#", ""), 16);
  return [value & 0xff, (value >> 8) & 0xff, (value >> 16) & 0xff, 0xff];
}

export const RING_COLOUR = bgra(token("--accent"));
const DIGIT_COLOUR = bgra(token("--fg-inverse"));
const OUTSET = px("--space-1");
const STROKE = 2;
const RADIUS = px("--radius-sm");
const TAG_HEIGHT = px("--leading-2xs");
const TAG_PAD = px("--space-1");

/** 0–9, five columns by seven rows, the top row first; the zero slashed, as a mono face draws it. */
const DIGITS: readonly (readonly string[])[] = [
  ["01110", "10001", "10011", "10101", "11001", "10001", "01110"],
  ["00100", "01100", "00100", "00100", "00100", "00100", "01110"],
  ["01110", "10001", "00001", "00010", "00100", "01000", "11111"],
  ["11111", "00010", "00100", "00010", "00001", "10001", "01110"],
  ["00010", "00110", "01010", "10010", "11111", "00010", "00010"],
  ["11111", "10000", "11110", "00001", "00001", "10001", "01110"],
  ["00110", "01000", "10000", "11110", "10001", "10001", "01110"],
  ["11111", "00001", "00010", "00100", "01000", "01000", "01000"],
  ["01110", "10001", "10001", "01110", "10001", "10001", "01110"],
  ["01110", "10001", "10001", "01111", "00001", "00010", "01100"],
];
const GLYPH_WIDTH = 5;
const GLYPH_HEIGHT = 7;

type Rect = { x0: number; y0: number; x1: number; y1: number };
type Corners = { tl: boolean; tr: boolean; bl: boolean; br: boolean };
const ALL: Corners = { tl: true, tr: true, bl: true, br: true };

/** Whether pixel (x, y)'s centre is inside `rect` with the named corners rounded to `radius`. */
function inside(x: number, y: number, rect: Rect, radius: number, corners: Corners): boolean {
  if (x < rect.x0 || x >= rect.x1 || y < rect.y0 || y >= rect.y1) return false;
  if (radius <= 0) return true;
  const cx = x + 0.5;
  const cy = y + 0.5;
  const left = cx < rect.x0 + radius;
  const right = cx > rect.x1 - radius;
  const top = cy < rect.y0 + radius;
  const bottom = cy > rect.y1 - radius;
  const corner = (top && left && corners.tl) || (top && right && corners.tr) || (bottom && left && corners.bl) || (bottom && right && corners.br);
  if (!corner) return true;
  const dx = cx - (left ? rect.x0 + radius : rect.x1 - radius);
  const dy = cy - (top ? rect.y0 + radius : rect.y1 - radius);
  return dx * dx + dy * dy <= radius * radius;
}

/**
 * Draw `ring` and `number` into a BGRA bitmap `width` by `height`, in place. **Held to the picture**: a box at its edge, or
 * the whole window, is ringed along the edge, and the tag moves in rather than leave it.
 */
export function drawRing(pixels: Uint8Array, width: number, height: number, ring: Ring, number: number): void {
  const scale = ring.scale > 0 ? ring.scale : 1;
  const put = (x: number, y: number, colour: readonly number[]): void => {
    const at = (y * width + x) * 4;
    pixels[at] = colour[0]!;
    pixels[at + 1] = colour[1]!;
    pixels[at + 2] = colour[2]!;
    pixels[at + 3] = colour[3]!;
  };
  const outset = Math.round(OUTSET * scale);
  const stroke = Math.max(1, Math.round(STROKE * scale));
  const radius = Math.round(RADIUS * scale);
  const outer: Rect = {
    x0: Math.max(0, ring.x - outset),
    y0: Math.max(0, ring.y - outset),
    x1: Math.min(width, ring.x + ring.width + outset),
    y1: Math.min(height, ring.y + ring.height + outset),
  };
  if (outer.x1 <= outer.x0 || outer.y1 <= outer.y0) return;
  const inner: Rect = { x0: outer.x0 + stroke, y0: outer.y0 + stroke, x1: outer.x1 - stroke, y1: outer.y1 - stroke };
  const innerRadius = Math.max(0, radius - stroke);
  for (let y = outer.y0; y < outer.y1; y += 1) {
    // Rows clear of the corners cross the stroke only at its two sides.
    const straight = y >= outer.y0 + radius + stroke && y < outer.y1 - radius - stroke;
    const columns = straight ? [outer.x0, outer.x0 + stroke, outer.x1 - stroke, outer.x1] : [outer.x0, outer.x1];
    for (let pair = 0; pair < columns.length; pair += 2) {
      for (let x = columns[pair]!; x < columns[pair + 1]!; x += 1) {
        if (inside(x, y, outer, radius, ALL) && !inside(x, y, inner, innerRadius, ALL)) put(x, y, RING_COLOUR);
      }
    }
  }

  // The tag: whole device pixels per glyph pixel, so the digits stay sharp.
  const unit = Math.max(1, Math.round(scale));
  const digits = String(Math.max(0, Math.floor(number)));
  const text = digits.length * (GLYPH_WIDTH + 1) * unit - unit;
  const pad = Math.round(TAG_PAD * scale);
  const tagWidth = text + 2 * pad;
  const tagHeight = Math.max(Math.round(TAG_HEIGHT * scale), GLYPH_HEIGHT * unit + 2 * unit);
  const tx = Math.max(0, Math.min(outer.x0, width - tagWidth));
  const ty = Math.max(0, Math.min(outer.y0, height - tagHeight));
  const tag: Rect = { x0: tx, y0: ty, x1: Math.min(width, tx + tagWidth), y1: Math.min(height, ty + tagHeight) };
  const pill: Corners = { tl: true, tr: false, bl: false, br: true };
  for (let y = tag.y0; y < tag.y1; y += 1) for (let x = tag.x0; x < tag.x1; x += 1) if (inside(x, y, tag, radius, pill)) put(x, y, RING_COLOUR);
  const gy = ty + Math.floor((tagHeight - GLYPH_HEIGHT * unit) / 2);
  for (const [index, digit] of [...digits].entries()) {
    const rows = DIGITS[Number(digit)]!;
    const gx = tx + pad + index * (GLYPH_WIDTH + 1) * unit;
    for (let row = 0; row < GLYPH_HEIGHT; row += 1) {
      for (let column = 0; column < GLYPH_WIDTH; column += 1) {
        if (rows[row]![column] !== "1") continue;
        for (let dy = 0; dy < unit; dy += 1) {
          for (let dx = 0; dx < unit; dx += 1) {
            const x = gx + column * unit + dx;
            const y = gy + row * unit + dy;
            if (x < width && y < height) put(x, y, DIGIT_COLOUR);
          }
        }
      }
    }
  }
}

/** Burns a ring and its number into a PNG: `burnRing`, and a test's stand-in. */
export type Burn = (png: Uint8Array, ring: Ring, number: number) => Uint8Array;

/** A PNG with `ring` and `number` burned in. A picture that cannot be read goes as it came: a plain picture beats none. */
export function burnRing(png: Uint8Array, ring: Ring, number: number): Buffer {
  const image = nativeImage.createFromBuffer(Buffer.from(png));
  const { width, height } = image.getSize();
  const bitmap = image.toBitmap();
  if (image.isEmpty() || bitmap.length !== width * height * 4) return Buffer.from(png);
  drawRing(bitmap, width, height, ring, number);
  return nativeImage.createFromBitmap(bitmap, { width, height }).toPNG();
}
