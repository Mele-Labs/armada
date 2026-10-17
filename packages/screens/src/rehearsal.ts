// The checkout's run sheet, read by the Manifest surface and by a Studio's run nodes.
//
// **Ids are prefixed by group**, so a press can tell a Check from a Command
// from a server apart without a second lookup — `setup:`, `check:`,
// `command:`, `server:`.

import type { ConsoleOutputProps } from "@armada/components";
import type { CheckoutRunFollowed, CheckoutRunRecord } from "@armada/protocol";

export const SETUP_PREFIX = "setup:";
export const CHECK_PREFIX = "check:";
export const COMMAND_PREFIX = "command:";
export const SERVER_PREFIX = "server:";

/** Whether an id names one of the entries `servers` composed. */
export function isServerEntry(id: string): boolean {
  return id.startsWith(SERVER_PREFIX);
}

/** The bare Manifest name an entry id carries, whatever its group. */
export function nameOf(id: string): string {
  const at = id.indexOf(":");
  return at === -1 ? id : id.slice(at + 1);
}

/** A group's `says`, left off where there is nothing to say. */
export function saying(says: string | undefined): { says?: string } {
  return says === undefined ? {} : { says };
}

/**
 * The run a window is following, as the sheet's output. `undefined` where it is
 * reading no run, or where what it is following is not the entry now
 * selected.
 *
 * **Keyed to the selection, so it cannot outlive it.** `CheckoutRunFollowed` is one
 * subscription and it does not clear itself the instant a person picks a
 * different row. Selecting a server and starting it was the case that surfaced
 * it: the serving bar came up live and correct while the pane beneath it still
 * read the Command run before it, because nothing had told `output` those two
 * facts were no longer about the same entry. `selectedName` absent is the one
 * case this does not gate — a run already under way when the sheet opens onto
 * no chosen row yet, which reads as "nothing to compare against" rather than
 * "compare against nothing."
 */
export function runOutputOf(followed: CheckoutRunFollowed, selectedName?: string): ConsoleOutputProps | undefined {
  if (followed.state !== "following") return undefined;
  if (selectedName !== undefined && followed.name !== selectedName) return undefined;
  return {
    rows: followed.lines.map((text, at) => ({ row: "line" as const, at: followed.fromLine + at, text })),
    region: { says: followed.name, path: followed.path },
    following: followed.ended === undefined,
    // Still running and silent so far: nothing to say. Finished and silent is the fact.
    ...(followed.ended === undefined ? {} : { emptyNote: "Printed nothing." }),
  };
}

/**
 * How a finished run ended, for its hue — the one reading both run surfaces
 * share, so a run reads the same wherever it appears. `passed` is the code it
 * expected; `failed` is any other ending, a signal or a spawn that never
 * started included. **A run somebody stopped is `stopped`**: a person ended
 * it, which is not a failure's reading.
 *
 * Still no verdict. Nothing here is stored.
 */
export function runOutcomeOf(
  record: Pick<CheckoutRunRecord, "exit_code" | "expect_exit_code" | "stopped">,
): "passed" | "failed" | "stopped" {
  if (record.stopped) return "stopped";
  return record.exit_code === record.expect_exit_code ? "passed" : "failed";
}
