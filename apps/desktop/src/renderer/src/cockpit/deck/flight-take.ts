// Two takes on a flight-controls grid that outgrows two rows, played in the mock on `?take=`: `wider` (the default) grows the columns until
// every tile is on two rows and wraps below that only where a column would be narrower than a tile; `pages` lays the tiles in pages of six
// columns and two rows that sit side by side and snap into view, with a dot for each page (`FlightBlock.tsx`). The owner's ask of 23 Oct 2026.
// Only one stays: the one he picks, and this file goes with the other.

export type FlightTake = "wider" | "pages";

/** The take on the page address; Bridge has no query, so it is `wider` there. */
export const flightTake = (): FlightTake => (new URLSearchParams(window.location.search).get("take") === "pages" ? "pages" : "wider");

/** A page holds six columns of two rows, or fewer columns while every tile fits on the one page. */
const PAGE_COLS = 6;

/** How many pages `drawn` tiles make and how many columns each page has. */
export function pagesOf(drawn: number): { count: number; cols: number } {
  const count = Math.max(1, Math.ceil(drawn / (PAGE_COLS * 2)));
  return { count, cols: count === 1 ? Math.max(1, Math.ceil(drawn / 2)) : PAGE_COLS };
}

/** The numbers the grid reads: `wider` has no cap on its columns, `pages` has its columns' share of the width and the columns of every page together. */
export function flightGrid(drawn: number, take: FlightTake): Record<string, number> {
  if (take === "wider") return { "--count": drawn, "--cols": Math.ceil(drawn / 2) };
  const { count, cols } = pagesOf(drawn);
  return { "--count": drawn, "--cols": cols, "--span": count * cols };
}

/** Where tile `at` of `drawn` stands in the `pages` take: its cell, and which page it is on, the first tile of each page being where a swipe snaps. Nothing in `wider`. */
export function pageCell(at: number, drawn: number, take: FlightTake): Record<string, unknown> {
  if (take === "wider") return {};
  const { cols } = pagesOf(drawn);
  const page = Math.floor(at / (cols * 2));
  const inPage = at % (cols * 2);
  return {
    style: { gridColumn: page * cols + (inPage % cols) + 1, gridRow: Math.floor(inPage / cols) + 1 },
    "data-page": page,
    "data-page-first": inPage === 0 || undefined,
  };
}
