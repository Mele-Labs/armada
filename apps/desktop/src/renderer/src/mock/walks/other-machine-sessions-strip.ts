// The other machine's waiting items on the Cockpit's Your move as a strip of single lines under the
// glass, so this machine's cards stay the only things with keys. Then that machine goes quiet. Over
// `other-machine-strip`.

import { inside, role, tab, text, walk } from "../walk";

const strip = role("list", "Waiting on the other machine");
const MACHINE = "MacBook Air";

const otherMachineSessionsStrip = walk("other-machine-strip", [
  { press: tab("Your move"), say: "Your move" },
  { look: inside(strip, text("Cap it at 5 or 8 tries?")), say: "The other machine's questions, a line each" },
  { hover: inside(strip, role("img", MACHINE)), say: "The laptop names the machine" },
  { later: inside(strip, role("img", MACHINE)), say: "The other machine stops being heard from" },
  { hover: inside(strip, role("img", /^Last heard/)), say: "The lines go quiet, and how long since it was last heard" },
]);

export { otherMachineSessionsStrip as "other-machine-sessions-strip" };
