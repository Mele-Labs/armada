// The home machine, signing in and coming to nothing four ways, then refused outright: the code is
// turned down, runs out, the forge cannot be reached, the forge says no, and last the sign-in app and
// the store address are blank. Each ended card has Try again; the last has a button to each setting.
// The walk `sign-in-ended` plays it.

import { signInGap } from "../sign-in-gaps";

export const signInEnded = signInGap("sign-in-ended", "Signing in comes to nothing four ways, then is refused for blank settings", {
  attempts: [{ ends: "denied" }, { ends: "expired" }, { ends: "unreachable" }, { ends: "refused" }, { blank: ["account", "store"] }],
});
