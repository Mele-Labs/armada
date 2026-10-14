// The other machine's waiting items on the Cockpit's Your move as cards the shape of this machine's,
// dashed and with nothing to press, under the card in front. The deck's keys never reach them. Then
// that machine goes quiet. Over `other-machine-cards`.

import { inside, region, role, tab, text, walk } from "../walk";

const card = (title: string) => region(`MacBook Air: ${title}`);
const MACHINE = "MacBook Air";

const otherMachineSessionsCards = walk("other-machine-cards", [
  { press: tab("Your move"), say: "Your move" },
  { look: inside(card("Bound the retry tries"), text("Cap it at 5 or 8 tries?")), say: "The other machine's questions, a card each" },
  { hover: inside(card("Bound the retry tries"), role("img", MACHINE)), say: "The laptop names the machine" },
  { later: inside(card("Pin the CI toolchain"), role("img", MACHINE)), say: "The other machine stops being heard from" },
  { hover: inside(card("Bound the retry tries"), role("img", /^Last heard/)), say: "The cards go quiet, and how long since it was last heard" },
]);

export { otherMachineSessionsCards as "other-machine-sessions-cards" };
