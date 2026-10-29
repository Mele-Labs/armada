// The ring and number burned into a note's picture as it is sent, read back pixel by pixel.

import { beforeEach, describe, expect, it, vi } from "vitest";

const native = vi.hoisted(() => ({ size: { width: 40, height: 30 }, empty: false, drawn: null as Uint8Array | null }));
vi.mock("electron", () => ({
  nativeImage: {
    createFromBuffer: () => ({
      getSize: () => native.size,
      isEmpty: () => native.empty,
      toBitmap: () => Buffer.alloc(native.size.width * native.size.height * 4, 0x80),
    }),
    createFromBitmap: (bitmap: Uint8Array) => {
      native.drawn = bitmap;
      return { toPNG: () => Buffer.from("burned") };
    },
  },
}));

import { burnRing, DIGIT_COLOUR, drawRing, RING_COLOUR } from "./burn";

/** A grey picture, `width` by `height`, as a BGRA bitmap. */
const grey = (width: number, height: number) => ({ width, height, pixels: new Uint8Array(width * height * 4).fill(0x80) });
type Picture = { width: number; height: number; pixels: Uint8Array };
const at = ({ width, pixels }: Picture, x: number, y: number): number[] => [...pixels.subarray((y * width + x) * 4, (y * width + x) * 4 + 4)];
const UNTOUCHED = [0x80, 0x80, 0x80, 0x80];

describe("a ring", () => {
  it("is drawn a little outside the box, 2px wide, and leaves the box itself as it was", () => {
    const shot = grey(200, 120);
    drawRing(shot.pixels, shot.width, shot.height, { x: 50, y: 40, width: 60, height: 30, scale: 1 }, 3);
    // Outset 4: the stroke is x 46–47 on the left, 112–113 on the right, y 72–73 at the foot.
    expect(at(shot, 46, 55)).toEqual(RING_COLOUR);
    expect(at(shot, 47, 55)).toEqual(RING_COLOUR);
    expect(at(shot, 48, 55)).toEqual(UNTOUCHED);
    expect(at(shot, 113, 55)).toEqual(RING_COLOUR);
    expect(at(shot, 111, 55)).toEqual(UNTOUCHED);
    expect(at(shot, 80, 73)).toEqual(RING_COLOUR);
    expect(at(shot, 80, 36)).toEqual(RING_COLOUR);
    // Inside the box, and outside the ring.
    expect(at(shot, 80, 55)).toEqual(UNTOUCHED);
    expect(at(shot, 108, 68)).toEqual(UNTOUCHED);
    expect(at(shot, 45, 55)).toEqual(UNTOUCHED);
    expect(at(shot, 80, 74)).toEqual(UNTOUCHED);
  });

  it("is scaled to the picture's device pixels", () => {
    const shot = grey(400, 240);
    drawRing(shot.pixels, shot.width, shot.height, { x: 100, y: 80, width: 120, height: 60, scale: 2 }, 1);
    // Outset 8 and stroke 4 at twice the CSS pixels.
    expect([92, 93, 94, 95].map((x) => at(shot, x, 110))).toEqual([RING_COLOUR, RING_COLOUR, RING_COLOUR, RING_COLOUR]);
    expect(at(shot, 96, 110)).toEqual(UNTOUCHED);
    expect(at(shot, 91, 110)).toEqual(UNTOUCHED);
  });

  it("runs along the picture's edge where the box is the whole window, and its tag stays on the picture", () => {
    const shot = grey(200, 120);
    drawRing(shot.pixels, shot.width, shot.height, { x: 0, y: 0, width: 200, height: 120, scale: 1 }, 2);
    expect(at(shot, 0, 60)).toEqual(RING_COLOUR);
    expect(at(shot, 1, 60)).toEqual(RING_COLOUR);
    expect(at(shot, 2, 60)).toEqual(UNTOUCHED);
    expect(at(shot, 199, 60)).toEqual(RING_COLOUR);
    expect(at(shot, 100, 119)).toEqual(RING_COLOUR);
    // The tag's first glyph row, at the picture's top-left corner: "2" opens 01110.
    expect(at(shot, 5, 4)).toEqual(DIGIT_COLOUR);
    expect(at(shot, 4, 4)).toEqual(RING_COLOUR);
  });
});

describe("a ring's tag", () => {
  // The tag sits at the ring's top-left corner, 46,36; its digits start 4 in and 4 down: 50,40.
  const tagged = (number: number): Picture => {
    const shot = grey(200, 120);
    drawRing(shot.pixels, shot.width, shot.height, { x: 50, y: 40, width: 60, height: 30, scale: 1 }, number);
    return shot;
  };

  it("draws the note's number in the ring's colour pill", () => {
    const three = tagged(3);
    // "3": 11111 across its top row, then 00010.
    expect([50, 51, 52, 53, 54].map((x) => at(three, x, 40))).toEqual(Array(5).fill(DIGIT_COLOUR));
    expect(at(three, 53, 41)).toEqual(DIGIT_COLOUR);
    expect(at(three, 50, 41)).toEqual(RING_COLOUR);
    // The pill behind it, 16 tall.
    expect(at(three, 55, 51)).toEqual(RING_COLOUR);
    expect(at(three, 55, 52)).toEqual(UNTOUCHED);
  });

  it("draws a different number differently, and every digit of one past 9", () => {
    const one = tagged(1);
    // "1" opens 00100.
    expect(at(one, 50, 40)).toEqual(RING_COLOUR);
    expect(at(one, 52, 40)).toEqual(DIGIT_COLOUR);
    const twelve = tagged(12);
    // "1" then "2", six pixels on: "2" opens 01110.
    expect(at(twelve, 52, 40)).toEqual(DIGIT_COLOUR);
    expect(at(twelve, 56, 40)).toEqual(RING_COLOUR);
    expect(at(twelve, 57, 40)).toEqual(DIGIT_COLOUR);
  });
});

describe("burning a PNG", () => {
  beforeEach(() => {
    native.size = { width: 40, height: 30 };
    native.empty = false;
    native.drawn = null;
  });

  it("decodes it, draws the ring into its bitmap and encodes it again", () => {
    expect(burnRing(Buffer.from("png"), { x: 10, y: 10, width: 10, height: 10, scale: 1 }, 1).toString()).toBe("burned");
    const drawn = { width: 40, height: 30, pixels: native.drawn! };
    expect(at(drawn, 6, 15)).toEqual(RING_COLOUR);
    expect(at(drawn, 19, 21)).toEqual(UNTOUCHED);
  });

  it("sends a picture it cannot read as it came", () => {
    native.empty = true;
    expect(burnRing(Buffer.from("not a png"), { x: 1, y: 1, width: 1, height: 1, scale: 1 }, 1).toString()).toBe("not a png");
    expect(native.drawn).toBeNull();
  });
});
