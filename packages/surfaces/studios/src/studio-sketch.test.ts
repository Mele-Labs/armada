import { describe, expect, it } from "vitest";

import type { SketchDrawing } from "@armada/protocol";

import { drawnAs, keptOf, padOf, toKeep } from "./studio-sketch";

const KEPT: SketchDrawing = {
  boxes: [{ id: "b1", x: 0, y: 0, body: "The rail" }],
  joins: [{ id: "b1-p1", from: "b1", to: "p1" }],
  strokes: [{ id: "s1", points: [{ x: 1, y: 2 }, { x: 3, y: 4 }] }],
  pictures: [
    {
      id: "p1",
      x: 10,
      y: 200,
      width: 320,
      height: 200,
      frame: { filename: "01S-01F.png", byte_size: 9, width: 1280, height: 800 },
    },
  ],
};

describe("a Studio's Sketch on the pad", () => {
  it("opens with every part, and a kept picture's bytes left to the board", () => {
    const pad = padOf(KEPT);
    expect(pad.shapes).toEqual([{ id: "b1", x: 0, y: 0, body: "The rail" }]);
    expect(pad.pictures).toEqual([{ id: "p1", x: 10, y: 200, width: 320, height: 200, src: "" }]);
    expect([...keptOf(KEPT)]).toEqual(["p1"]);
  });

  it("is unchanged until something on it moves, whatever a picture's src became", () => {
    const pad = padOf(KEPT);
    const shown = { ...pad, pictures: pad.pictures.map((one) => ({ ...one, src: "blob:x" })) };
    expect(drawnAs(shown)).toBe(drawnAs(pad));
    const moved = { ...pad, shapes: [{ ...pad.shapes[0]!, x: 40.4 }] };
    expect(drawnAs(moved)).not.toBe(drawnAs(pad));
  });

  it("names a kept picture and sends a new one's bytes, at whole places", async () => {
    const pad = padOf(KEPT);
    const pasted = { id: "p2", x: 0.6, y: 5, width: 100, height: 60, src: "blob:pasted" };
    const read: string[] = [];
    const sent = await toKeep(
      { ...pad, shapes: [{ ...pad.shapes[0]!, x: 12.7 }], pictures: [...pad.pictures, pasted] },
      keptOf(KEPT),
      async (src) => {
        read.push(src);
        return new Uint8Array([1, 2, 3]);
      },
    );
    expect(sent.boxes[0]).toEqual({ id: "b1", x: 13, y: 0, body: "The rail" });
    expect(sent.pictures[0]).toEqual({ id: "p1", x: 10, y: 200, width: 320, height: 200 });
    expect(sent.pictures[1]).toEqual({
      id: "p2",
      x: 1,
      y: 5,
      width: 100,
      height: 60,
      bytes: new Uint8Array([1, 2, 3]),
    });
    expect(read).toEqual(["blob:pasted"]);
  });
});
