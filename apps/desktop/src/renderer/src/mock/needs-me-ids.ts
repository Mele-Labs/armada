// Kept outside scenarios/: every export there is read as a Scenario.

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

/** Its turn ended on a message to him, 6 minutes ago. */
export const REPLIED_AT = ago(6);
/** The ids Fleet gives the reply and the done as items, for a test that dismisses them. */
export const REPLIED_ITEM = `replied:${REPLIED_AT}`;
/** It said nothing is left, 4 minutes ago. */
export const DONE_AT = ago(4);
export const DONE_ITEM = `done:${DONE_AT}`;
