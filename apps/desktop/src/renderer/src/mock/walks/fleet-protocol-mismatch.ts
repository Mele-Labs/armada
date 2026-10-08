// Fleet and Bridge on different protocols: what a person is told, and the one
// thing to do. The older-Fleet wording, for a runtime file with no ID, is
// `packages/shell/src/fleet.ts`.

import { text, walk } from "../walk";

export const fleetProtocolMismatch = walk("fleet/protocol-mismatch", [
  { look: text("Fleet and Bridge do not match"), say: "The notice names both sides" },
  { look: text("Run /update-armada, then reopen Bridge."), say: "One thing to do" },
]);
