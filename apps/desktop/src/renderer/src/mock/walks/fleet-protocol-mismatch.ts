// Fleet and Bridge on different protocols: what a person is told, and the one
// thing to press. Three rows of `scenarios/fleet-protocol-mismatch.ts`: the
// restart that works, the one that brings back a Fleet still apart, and a
// Fleet launchd does not hold. The older-Fleet wording, for a runtime file with
// no ID, is `packages/shell/src/fleet.ts`.

import { button, role, text, walk } from "../walk";

const RESTART = button("Restart Fleet");

export const fleetProtocolMismatch = walk("fleet/protocol-mismatch", [
  { look: text("Fleet and Bridge do not match"), say: "The notice names both sides" },
  { look: text("Restart Fleet onto the installed build."), say: "One thing to do" },
  { hover: RESTART, say: "The tooltip says what a working Drone costs" },
  { press: RESTART, say: "Working: the button shows it is restarting and cannot be pressed again" },
  { look: role("img", "Fleet — Running", { exact: true }), say: "Fleet came back matching, so the notice is gone" },
]);

export const fleetProtocolMismatchStillApart = walk("fleet/protocol-mismatch-still-apart", [
  { press: RESTART, say: "Restart" },
  { look: text("Still not matching"), say: "Fleet is back on the installed build and it is not this Bridge's" },
]);

export const fleetProtocolMismatchUnmanaged = walk("fleet/protocol-mismatch-unmanaged", [
  { press: RESTART, say: "Restart" },
  { look: text("Fleet was not started by Armada's service"), say: "Nothing was restarted, and it says why" },
]);
