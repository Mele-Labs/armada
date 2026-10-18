// Release on one row of a Session's ledger: hovering a held row shows it, a confirm names the row (a branch row with the slot it carries), Cancel changes nothing, and Release
// gives it back and leaves it drawn given back. Over `session-ledger-release-row`.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const ledger = region("Attachments");
const branch = inside(ledger, role("listitem", "Branch fix/pin-store-clock"));
const release4 = inside(branch, button("Release fix/pin-store-clock"));
const asked4 = dialog("Release fix/pin-store-clock in slot-4");

const sessionLedgerReleaseRow = walk("session-ledger-release-row", [
  { press: deckRow("Pin the store clock"), say: "A Job open on the stage, working, its ledger at the side" },
  { hover: branch, say: "Hovering a held row shows Release at its end, the unplug glyph. This branch row carries slot 4" },
  { hover: release4, say: "Its tooltip names the row: Release fix/pin-store-clock" },
  { press: release4, say: "A confirm names the branch and the slot it carries, and says what happens to them" },
  { press: inside(asked4, button("Cancel")), say: "Cancel closes it and changes nothing" },
  { look: release4, say: "The row is held as it was" },
  { press: release4, say: "Release again" },
  { press: inside(asked4, button("Release", { exact: true })), say: "Fleet gives the slot back, with the branch it has checked out" },
  { look: inside(branch, role("img", "Given back")), say: "The row stays, given back, with no Release on it" },
]);

export { sessionLedgerReleaseRow as "session-ledger-release-row" };
