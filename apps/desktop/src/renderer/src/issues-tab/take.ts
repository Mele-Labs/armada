// Two takes on how the Issues list and reader are drawn, played in the mock on `?take=`. `cards` (the default) draws each issue
// as a card on a fill, a Job's state on its frame and left edge and the Job named under the title, the reader's parts each
// a raised panel. `rows` draws a dense table, an issue a row on one hairline, the Job's state on the left edge alone, the
// reader one flat page. The owner's notes of 23 Oct 2026. Only one stays: the one he picks, and this file goes with the other.

export type IssuesTake = "cards" | "rows";

/** The take on the page address; Bridge has no query, so it is `cards` there. */
export const issuesTake = (): IssuesTake => (new URLSearchParams(window.location.search).get("take") === "rows" ? "rows" : "cards");
