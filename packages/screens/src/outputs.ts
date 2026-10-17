// What a Check log says when it could not be followed: a failure, never a stand-in for lines that have not
// arrived. The sentences are the app's voice for the three ways a followed log goes wrong.

/**
 * What a followed log with no lines says, **only where something went wrong**: Fleet did not
 * answer, could not read the file, or the connection went. `undefined` otherwise, for the log
 * panel, where a log still opening or one with nothing in it yet is an empty slot and stays empty.
 */
export function followFailedNote(failed: boolean, ended: string | undefined): string | undefined {
  if (failed) return NOT_ANSWERED_LIVE;
  if (ended === "unreadable") return UNREADABLE_LIVE;
  if (ended === "broke") return CLOSED_LIVE;
  return undefined;
}

/** Fleet said it could not read the file. */
const UNREADABLE_LIVE = "Fleet could not read this Check's log.";

/** The connection went without Fleet saying why. */
const CLOSED_LIVE = "The connection to this Check's log closed.";

/** Fleet did not answer the ask to follow it. */
const NOT_ANSWERED_LIVE = "Fleet did not answer for this Check's log.";
