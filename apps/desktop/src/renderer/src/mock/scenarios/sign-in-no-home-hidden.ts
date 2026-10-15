// The second take of `sign-in-no-home`: with no shared home, What syncs is not drawn on an added profile.
// The walk `sign-in-no-home-hidden` plays it.

import { signInGap } from "../sign-in-gaps";

export const signInNoHomeHidden = signInGap("sign-in-no-home-hidden", "Signing in with no shared home yet: What syncs not drawn on an added profile", {
  noHome: "hidden",
});
