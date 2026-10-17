import { describe, expect, it, vi } from "vitest";

vi.mock("electron", () => ({ app: { getPath: () => "/tmp" }, screen: {} }));

import { place } from "./window-state";

const primary = { x: 0, y: 25, width: 1920, height: 1055 };
const second = { x: 1920, y: 0, width: 1920, height: 1080 };

describe("place", () => {
  it("keeps a saved rectangle that is on a display", () => {
    const saved = { x: 2000, y: 100, width: 1000, height: 700 };
    expect(place(saved, [primary, second])).toEqual(saved);
  });

  it("centres the default size on the primary display when the monitor is gone", () => {
    const saved = { x: 2000, y: 100, width: 1000, height: 700 };
    expect(place(saved, [primary])).toEqual({ x: 320, y: 153, width: 1280, height: 800 });
  });

  it("does the same with nothing saved, and shrinks to a small display", () => {
    expect(place(null, [{ x: 0, y: 0, width: 1000, height: 600 }])).toEqual({ x: 0, y: 0, width: 1000, height: 600 });
  });
});
