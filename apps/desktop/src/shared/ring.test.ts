// Where a note's box is on its picture: the window it was taken of, in that picture's own pixels.

import { describe, expect, it } from "vitest";

import { isRing, pngSize, ringOn } from "./ring";

/** A PNG's signature and header, `width` by `height`. */
function header(width: number, height: number): Uint8Array {
  const bytes = new Uint8Array(24);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]);
  new DataView(bytes.buffer).setUint32(16, width);
  new DataView(bytes.buffer).setUint32(20, height);
  return bytes;
}

describe("a picture's size", () => {
  it("is read off its PNG header, and is none for bytes that are not one", () => {
    expect(pngSize(header(2560, 1600))).toEqual({ width: 2560, height: 1600 });
    expect(pngSize(new Uint8Array([137, 80, 78, 71]))).toBeNull();
    expect(pngSize(new Uint8Array(24))).toBeNull();
  });
});

describe("a box on a picture", () => {
  it("is scaled by how much wider the picture is than the window, and keeps that scale", () => {
    expect(ringOn({ x: 254, y: 184, width: 557, height: 200 }, 1280, 2560)).toEqual({ x: 508, y: 368, width: 1114, height: 400, scale: 2 });
    expect(ringOn({ x: 10.4, y: 20.6, width: 30, height: 40 }, 1280, 1280)).toEqual({ x: 10, y: 21, width: 30, height: 40, scale: 1 });
  });

  it("is a ring only with every number in it", () => {
    expect(isRing({ x: 1, y: 2, width: 3, height: 4, scale: 2 })).toBe(true);
    expect(isRing({ x: 1, y: 2, width: 3, height: 4 })).toBe(false);
    expect(isRing({ x: "1", y: 2, width: 3, height: 4, scale: 1 })).toBe(false);
    expect(isRing(null)).toBe(false);
  });
});
