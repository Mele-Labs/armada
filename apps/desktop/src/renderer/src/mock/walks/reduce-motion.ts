// Reduce motion: Armada's own switch, in Settings beside Interface sounds. Its effect needs the real app: the mock's page cannot be told to reduce motion.
// Over the `cockpit-sessions-list` scenario.

import { role, toSystemPage, walk } from "../walk";

const reduceMotion = walk("cockpit-sessions-list", [
  ...toSystemPage("Features", "Features"),
  { look: role("switch", /^Reduce motion/), say: "Reduce motion is off until you turn it on. On, Armada's animations stop and the Mac's own setting stays as it is" },
  { press: role("switch", /^Reduce motion/), say: "On. It saves at once and takes effect with no restart. This mock cannot show the effect, so read it on the preview" },
]);

export { reduceMotion as "reduce-motion" };
