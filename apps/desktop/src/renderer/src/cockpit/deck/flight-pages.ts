// How the flight controls are laid out when a Job has more than two rows of them: in pages of six columns and two rows that sit side by side and
// snap into view, with a dot for each page (`FlightBlock.tsx`). While every tile fits on one page it has as many columns as it needs. The owner's ask of 23 Oct 2026.

import type { CSSProperties } from "react";

/** A page holds six columns of two rows, or fewer columns while every tile fits on the one page. */
const PAGE_COLS = 6;

/** How many pages `drawn` tiles make and how many columns each page has. */
export function pagesOf(drawn: number): { count: number; cols: number } {
  const count = Math.max(1, Math.ceil(drawn / (PAGE_COLS * 2)));
  return { count, cols: count === 1 ? Math.max(1, Math.ceil(drawn / 2)) : PAGE_COLS };
}

/** What the grid reads: the columns of a page and the columns of every page together, from which each column's share of the width follows. */
export function flightGrid(drawn: number): CSSProperties {
  const { count, cols } = pagesOf(drawn);
  return { "--cols": cols, "--span": count * cols } as CSSProperties;
}

/** Where tile `at` of `drawn` stands: its cell, and which page it is on, the first tile of each page being where a swipe snaps. */
export function pageCell(at: number, drawn: number): { style: CSSProperties; "data-page": number; "data-page-first": true | undefined } {
  const { cols } = pagesOf(drawn);
  const page = Math.floor(at / (cols * 2));
  const inPage = at % (cols * 2);
  return { style: { gridColumn: page * cols + (inPage % cols) + 1, gridRow: Math.floor(inPage / cols) + 1 }, "data-page": page, "data-page-first": inPage === 0 || undefined };
}
