// Two takes on a flight-controls grid that outgrows two rows, played in the mock on `?take=`: `wider` (the default) grows the columns until
// every tile is on two rows and wraps below that only where a column would be narrower than a tile; `peek` keeps six columns and marks that
// tiles sit below with a chevron (`MoreBelow.tsx`). The owner's ask of 23 Oct 2026. Only one stays: the one he picks, and this file goes with the other.

export type FlightTake = "wider" | "peek";

/** The take on the page address; Bridge has no query, so it is `wider` there. */
export const flightTake = (): FlightTake => (new URLSearchParams(window.location.search).get("take") === "peek" ? "peek" : "wider");

/** How many columns and how many tiles in a row the grid is told, by the take: `wider` has no cap, `peek` stops at six columns and twelve tiles. */
export function flightGrid(drawn: number, take: FlightTake): { "--count": number; "--cols": number } {
  const cap = take === "wider" ? Infinity : 12;
  const across = take === "wider" ? Infinity : 6;
  return { "--count": Math.min(drawn, cap), "--cols": Math.min(Math.ceil(drawn / 2), across) };
}
