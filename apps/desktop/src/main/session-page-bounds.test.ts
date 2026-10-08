// Where the page view is placed is the renderer's number held inside the window.

import { describe, expect, it } from "vitest";

import { boundsOf, isEscape } from "./session-page-bounds";

const window = { width: 1280, height: 800 };

describe("the page view's bounds", () => {
  it("rounds what the renderer reported", () => {
    expect(boundsOf({ x: 100.4, y: 50.6, width: 400.2, height: 300.5 }, window)).toEqual({ x: 100, y: 51, width: 400, height: 301 });
  });

  it("keeps a rect that runs past the window inside it", () => {
    expect(boundsOf({ x: 1000, y: 700, width: 900, height: 900 }, window)).toEqual({ x: 1000, y: 700, width: 280, height: 100 });
  });

  it("holds a negative, infinite or missing number to nothing", () => {
    expect(boundsOf({ x: -5, y: Infinity, width: NaN, height: -1 }, window)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
    expect(boundsOf(undefined, window)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
    expect(boundsOf("everything", window)).toEqual({ x: 0, y: 0, width: 0, height: 0 });
  });
});

describe("Esc in the page view", () => {
  it("is Esc going down, and nothing else", () => {
    expect(isEscape({ type: "keyDown", key: "Escape" })).toBe(true);
    expect(isEscape({ type: "keyUp", key: "Escape" })).toBe(false);
    expect(isEscape({ type: "keyDown", key: "a" })).toBe(false);
  });
});
