// Kept outside scenarios/: every export there is read as a Scenario.

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** Its turn ended on a message to him, 6 minutes ago. */
export const REPLIED_AT = ago(6);
/** The id Fleet gives the done as an item, for a test that dismisses it. */
/** Its turn ended on a question to him, 7 minutes ago, and the item Fleet raises for the line that asks. */
export const ASKED_AT = ago(7);
export const ASKED_ITEM = `replied:${ASKED_AT}`;
/** It said nothing is left, 4 minutes ago. */
export const DONE_AT = ago(4);
export const DONE_ITEM = `done:${DONE_AT}`;
