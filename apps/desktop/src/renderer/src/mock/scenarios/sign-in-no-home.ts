// The home machine signing in where no shared home exists yet: the third set-up row is not set up
// instead of turning, and a profile added after it has What syncs greyed, with the reason on hover.
// The walk `sign-in-no-home` plays it; `sign-in-no-home-hidden` is the same with the button not drawn.

import { signInGap } from "../sign-in-gaps";

export const signInNoHome = signInGap("sign-in-no-home", "Signing in with no shared home yet: What syncs greyed on an added profile", {
  noHome: "disabled",
});
