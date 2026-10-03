/**
 * Where a proposed relation's dot sits on its line — the owner, 2 Oct 2026:
 * *"The label stops covering the cards it runs between: it moves along its
 * line, or shrinks to a dot that opens on hover."*
 *
 * **Both, because the dot alone is not enough.** A line drawn straight across
 * a third card puts its middle on that card, so a dot left at the middle sits
 * on the card's text. A dot needs a step of clear line, which the gap between
 * two cards nearly always has, so it moves to the clear point nearest the
 * middle. Where the whole line is covered it stays at the middle.
 */

type At = { x: number; y: number };

/** A card on the board, in the board's own units. */
export type CardBox = { x: number; y: number; width: number; height: number };

/** How far round the dot a card is kept: half the dot's hit area, and a step. */
const DOT_ROOM = 12;

/** How many points along the line are tried, either side of the middle. */
const TRIES = 24;

/** The four points of React Flow's bezier, `M x,y C x,y x,y x,y`, or nothing for another shape. */
function pointsOf(path: string): number[] | null {
  const numbers = path.match(/-?\d+(?:\.\d+)?(?:e[-+]?\d+)?/gi)?.map(Number) ?? [];
  return numbers.length === 8 ? numbers : null;
}

export function dotOnTheLine(path: string, middle: At, cards: readonly CardBox[]): At {
  const clear = (at: At) =>
    !cards.some(
      (card) =>
        at.x > card.x - DOT_ROOM &&
        at.x < card.x + card.width + DOT_ROOM &&
        at.y > card.y - DOT_ROOM &&
        at.y < card.y + card.height + DOT_ROOM,
    );
  if (clear(middle)) return middle;
  const points = pointsOf(path);
  if (points === null) return middle;
  const [x0, y0, x1, y1, x2, y2, x3, y3] = points as [number, number, number, number, number, number, number, number];
  const on = (t: number): At => {
    const u = 1 - t;
    return {
      x: u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3,
      y: u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3,
    };
  };
  for (let step = 1; step < TRIES; step += 1) {
    for (const t of [0.5 - step / (2 * TRIES), 0.5 + step / (2 * TRIES)]) {
      const at = on(t);
      if (clear(at)) return at;
    }
  }
  return middle;
}
