// Release on one row of a Session's ledger: hovering a held row shows it, a confirm names the row, Cancel changes nothing, and Release
// gives it back and leaves it drawn given back. A row a piloted Job handed over is kept, and says why. Over `session-ledger-release-row`.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const ledger = region("Attachments");
const slot4 = inside(ledger, role("listitem", "Worktree slot 4"));
const slot7 = inside(ledger, role("listitem", "Worktree slot 7"));
const release4 = inside(slot4, button("Release slot-4"));
const release7 = inside(slot7, button("Release slot-7"));
const asked4 = dialog("Release slot-4");
const asked7 = dialog("Release slot-7");

const sessionLedgerReleaseRow = walk("session-ledger-release-row", [
  { press: deckRow("Pin the store clock"), say: "A Session open on the stage, working, its ledger at the side" },
  { hover: slot4, say: "Hovering a held row shows Release at its end, the unplug glyph" },
  { hover: release4, say: "Its tooltip names the row: Release slot-4" },
  { press: release4, say: "A confirm names the one row and says what happens to it" },
  { press: inside(asked4, button("Cancel")), say: "Cancel closes it and changes nothing" },
  { look: release4, say: "The row is held as it was" },
  { press: release4, say: "Release again" },
  { press: inside(asked4, button("Release", { exact: true })), say: "Fleet gives the slot back, with the branch it has checked out" },
  { look: inside(slot4, role("img", "Given back")), say: "The row stays, given back, with no Release on it" },
  { look: inside(ledger, role("listitem", "Branch fix/pin-store-clock")), say: "Its branch went with it and reads given back too" },
  { hover: slot7, say: "A slot a piloted Job handed over" },
  { press: release7, say: "Release on it" },
  { press: inside(asked7, button("Release", { exact: true })), say: "Fleet keeps it" },
  { look: inside(slot7, text("A piloted Job holds it, and the pilot's exit gives it back.")), say: "The row says why it stayed, in Fleet's words" },
]);

export { sessionLedgerReleaseRow as "session-ledger-release-row" };
