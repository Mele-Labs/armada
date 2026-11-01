import { describe, expect, it } from "vitest";

import type { Box } from "../../../shared/annotations";
import { placeCard, type CardSize, type Metrics, type Viewport } from "./place";

/** The tokens `annotate.css` declares: `--space-2`, `--h-badge`, `--space-3`. */
const M: Metrics = { gap: 8, pinSize: 20, pinInset: 12 };

/** The owner's window, less the layer's own status bar at the foot of it. */
const OWNER: Viewport = { width: 1324, height: 922, floor: 880 };

/** The note card as it draws with a text box and two buttons. */
const CARD: CardSize = { width: 340, height: 336 };

const box = (x: number, y: number, width: number, height: number): Box => ({ x, y, width, height });

/** Everything the card draws is inside the window on all four edges. */
function inside(at: { x: number; y: number; maxHeight: number }, card: CardSize, view: Viewport): void {
  const height = Math.min(card.height, at.maxHeight);
  expect(at.x).toBeGreaterThanOrEqual(0);
  expect(at.y).toBeGreaterThanOrEqual(0);
  expect(at.x + card.width).toBeLessThanOrEqual(view.width);
  expect(at.y + height).toBeLessThanOrEqual(view.height);
}

describe("placeCard", () => {
  it("puts the card under an element with room under it", () => {
    const at = placeCard(box(200, 100, 480, 120), CARD, OWNER, M);
    expect(at.side).toBe("below");
    expect(at.y).toBe(228);
    expect(at.x).toBe(200);
    inside(at, CARD, OWNER);
  });

  it("puts it over an element with room only above it", () => {
    const at = placeCard(box(200, 500, 480, 340), CARD, OWNER, M);
    expect(at.side).toBe("above");
    // Clear of the pin on the element's top corner, not just of the element.
    expect(at.y + CARD.height).toBe(500 - M.pinSize / 2 - M.gap);
    inside(at, CARD, OWNER);
  });

  // The defect: the Job settings sheet is the height of the window, so neither
  // side has room and the card used to be drawn off the top edge. Now the card
  // goes beside it, so the tools that restyle the element never hide the change.
  it("puts the card beside an element as tall as the window, trailing side first, clear of its pin", () => {
    const element = box(432, 60, 480, 862);
    const at = placeCard(element, CARD, OWNER, M);
    expect(at.side).toBe("beside");
    // Past the element's trailing edge and the pin centred on its trailing corner, not over either.
    expect(at.x).toBe(432 + 480 + M.pinSize / 2 + M.gap);
    expect(at.y).toBe(60);
    expect(at.x).toBeGreaterThanOrEqual(element.x + element.width + M.pinSize / 2);
    inside(at, CARD, OWNER);
  });

  it("puts it on the leading side when the trailing side has no room", () => {
    const at = placeCard(box(900, 60, 380, 862), CARD, OWNER, M);
    expect(at.side).toBe("beside");
    expect(at.x).toBe(900 - M.gap - CARD.width);
    expect(at.x + CARD.width).toBeLessThanOrEqual(900);
    inside(at, CARD, OWNER);
  });

  it("keeps a card beside the element above the status bar, level with the element's top where it can be", () => {
    const short: Viewport = { width: 1324, height: 500, floor: 458 };
    const level = placeCard(box(432, 8, 480, 460), CARD, short, M);
    expect(level.side).toBe("beside");
    expect(level.y).toBe(8);
    // Lower than the card can stand, it is lifted to end at the floor, not drawn under the bar.
    const low = placeCard(box(432, 150, 480, 340), CARD, short, M);
    expect(low.side).toBe("beside");
    expect(low.y + CARD.height).toBe(short.floor - M.gap);
    inside(low, CARD, short);
  });

  it("still prefers below, then above, to beside", () => {
    expect(placeCard(box(200, 100, 480, 120), CARD, OWNER, M).side).toBe("below");
    expect(placeCard(box(200, 500, 480, 340), CARD, OWNER, M).side).toBe("above");
  });

  // Over is the last resort now: an element with no room beside it either, which is a sheet across the window.
  it("keeps the card on screen for an element as wide and as tall as the window, over it", () => {
    const at = placeCard(box(8, 60, 1308, 862), CARD, OWNER, M);
    expect(at.side).toBe("over");
    inside(at, CARD, OWNER);
  });

  it("clears the element's own pin when it has to cover the element", () => {
    const at = placeCard(box(8, 60, 1308, 862), CARD, OWNER, M);
    // The pin is centred on the element's top trailing corner: y 60, 20 tall.
    expect(at.y).toBeGreaterThanOrEqual(70 + M.gap);
  });

  it("stays above the layer's own status bar", () => {
    const at = placeCard(box(200, 500, 480, 60), CARD, OWNER, M);
    expect(at.y + CARD.height).toBeLessThanOrEqual(OWNER.floor);
  });

  it("caps a card taller than the window, which then scrolls inside itself", () => {
    const short: Viewport = { width: 1324, height: 320, floor: 278 };
    const at = placeCard(box(8, 60, 1308, 260), CARD, short, M);
    expect(at.side).toBe("over");
    expect(at.maxHeight).toBeLessThan(CARD.height);
    inside(at, CARD, short);
  });

  it("clamps the trailing edge in, as the stylesheet used to do alone", () => {
    const at = placeCard(box(1200, 100, 100, 40), CARD, OWNER, M);
    expect(at.x + CARD.width).toBe(OWNER.width - M.gap);
  });

  // Every window a person can drag to, against every element they can pick,
  // including elements off the top or bottom of the window after a scroll.
  it("is inside the window for any element in any window", () => {
    for (const height of [200, 320, 480, 700, 922, 1600]) {
      for (const width of [400, 768, 1324, 2200]) {
        const view: Viewport = { width, height, floor: Math.max(height - 42, 0) };
        for (const y of [-400, -20, 0, 40, height / 2, height - 30, height + 100]) {
          for (const h of [8, 120, height, height * 2]) {
            const at = placeCard(box(-50, y, 480, h), CARD, view, M);
            inside(at, CARD, view);
          }
        }
      }
    }
  });
});
