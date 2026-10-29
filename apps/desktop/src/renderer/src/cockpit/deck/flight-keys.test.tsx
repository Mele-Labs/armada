// A press on a flight tile first scrolls it into its row's view, so the person sees what the key did. Browser: a scroll position is what is read.

import { afterEach, expect, test, vi } from "vitest";

import { showTile } from "./flight-keys";

afterEach(() => {
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

test("showTile asks for the nearest edge, so a tile already in view does not move", () => {
  const tile = document.createElement("button");
  const scroll = vi.fn();
  tile.scrollIntoView = scroll;
  showTile(tile);
  expect(scroll).toHaveBeenCalledOnce();
  expect(scroll).toHaveBeenCalledWith(expect.objectContaining({ block: "nearest", inline: "nearest" }));
});

test("showTile scrolls a tile past the row's edge only as far as that edge, and leaves one in view where it is", () => {
  const row = document.createElement("div");
  row.style.cssText = "display: flex; width: 100px; overflow-x: auto; scroll-behavior: auto";
  const tiles = Array.from({ length: 5 }, () => {
    const one = document.createElement("button");
    one.style.cssText = "flex: none; width: 60px; height: 20px";
    return one;
  });
  row.append(...tiles);
  document.body.append(row);

  showTile(tiles[0]!);
  expect(row.scrollLeft).toBe(0);
  showTile(tiles[2]!);
  expect(tiles[2]!.getBoundingClientRect().right).toBeCloseTo(row.getBoundingClientRect().right, 0);
  const at = row.scrollLeft;
  showTile(tiles[2]!);
  expect(row.scrollLeft).toBe(at);
});
