// The sketch as a PNG, read back pixel by pixel.
//
// **A browser test, because the writer paints**: a canvas, a decoded picture
// and the tokens' own values, none of which node has. What it asserts is
// where things landed and what colour they are, so a picture left out, a box
// in the wrong place or a frame the wrong size each fail on their own.

import "@armada/tokens/tokens.css";

import { beforeAll, describe, expect, it } from "vitest";

import type { Drawing } from "@armada/screens/src/draft/sketch";
import { sketchInkOf, sketchPng, type SketchInk, type SketchPng } from "./sketch-png";

/** Pixels to each unit of the pad. */
const SCALE = 2;

/** What the picture is filled with, so where it was drawn can be read back. */
const PICTURE_FILL = "red";

/** One colour, as the canvas would store it. */
function rgbOf(colour: string): [number, number, number] {
  const pen = new OffscreenCanvas(1, 1).getContext("2d")!;
  pen.fillStyle = colour;
  pen.fillRect(0, 0, 1, 1);
  const [r, g, b] = pen.getImageData(0, 0, 1, 1).data;
  return [r!, g!, b!];
}

/** A picture of one colour, as a paste would hand the pad one: a `blob:` address. */
async function solid(width: number, height: number): Promise<string> {
  const canvas = new OffscreenCanvas(width, height);
  const pen = canvas.getContext("2d")!;
  pen.fillStyle = PICTURE_FILL;
  pen.fillRect(0, 0, width, height);
  return URL.createObjectURL(await canvas.convertToBlob({ type: "image/png" }));
}

/** The PNG decoded again, so it is read the way anything reading the file would. */
async function decode(png: SketchPng): Promise<ImageData> {
  const bitmap = await createImageBitmap(new Blob([png.bytes], { type: "image/png" }));
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const pen = canvas.getContext("2d")!;
  pen.drawImage(bitmap, 0, 0);
  return pen.getImageData(0, 0, bitmap.width, bitmap.height);
}

let ink: SketchInk;
let drawing: Drawing;
let png: SketchPng;
let read: ImageData;

/** The pixel at a place on the pad, through the frame's margin and the scale. */
function at(x: number, y: number): [number, number, number] {
  const px = Math.round((x + ink.margin) * SCALE);
  const py = Math.round((y + ink.margin) * SCALE);
  const from = (py * read.width + px) * 4;
  return [read.data[from]!, read.data[from + 1]!, read.data[from + 2]!];
}

function near(got: [number, number, number], want: string, slack = 2): boolean {
  return rgbOf(want).every((one, channel) => Math.abs(one - got[channel]!) <= slack);
}

beforeAll(async () => {
  ink = sketchInkOf(getComputedStyle(document.documentElement));
  drawing = {
    // One line of words, so the box is its two-row well and the inset round it.
    shapes: [{ id: "b1", x: 0, y: 0, body: "the stat" }],
    joins: [{ id: "b1-p1", from: "b1", to: "p1" }],
    // A line under the box, well clear of everything else.
    strokes: [
      {
        id: "s1",
        points: [
          { x: 20, y: 120 },
          { x: 120, y: 120 },
          { x: 220, y: 120 },
        ],
      },
    ],
    pictures: [{ id: "p1", x: 400, y: 0, width: 200, height: 150, src: await solid(200, 150) }],
  };
  png = await sketchPng(drawing, ink, SCALE);
  read = await decode(png);
});

describe("the sketch as a PNG", () => {
  it("is a PNG, framed on what is drawn with the margin round it", () => {
    expect([...png.bytes.slice(1, 4)].map((one) => String.fromCharCode(one)).join("")).toBe("PNG");
    // The picture's far edge is the drawing's right and bottom; the box's corner its left and top.
    expect(png.width).toBe((600 + ink.margin * 2) * SCALE);
    expect(png.height).toBe((150 + ink.margin * 2) * SCALE);
    expect([read.width, read.height]).toEqual([png.width, png.height]);
  });

  it("draws the pasted picture at its own place and size", () => {
    expect(near(at(500, 75), PICTURE_FILL)).toBe(true);
    expect(near(at(410, 10), PICTURE_FILL)).toBe(true);
    expect(near(at(590, 140), PICTURE_FILL)).toBe(true);
    // And nothing of it past its edge.
    expect(near(at(500, 160), ink.canvas)).toBe(true);
  });

  it("draws the box as the pad does: its fill, the well inside, the words in the well", () => {
    expect(near(at(4, 4), ink.boxFill)).toBe(true);
    // The second row is empty, so it is the well's own colour.
    expect(near(at(200, 55), ink.well)).toBe(true);
    // Somewhere along the first row the words are drawn.
    const row = Array.from({ length: 100 }, (_, step) => at(17 + step, 28));
    expect(row.some((pixel) => !near(pixel, ink.well, 24))).toBe(true);
  });

  it("draws the join between the box and the picture, and the line drawn by hand", () => {
    // The bezier's own middle, between the box's right side and the picture's left.
    expect(near(at(320, 57), ink.canvas, 8)).toBe(false);
    expect(near(at(120, 120), ink.pen, 8)).toBe(true);
  });

  it("leaves the margin the canvas's colour", () => {
    expect(near(at(-12, -12), ink.canvas)).toBe(true);
  });
});
