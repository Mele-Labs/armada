// The second take of `account-page`: the account is a panel opened from a row in the left rail. The walk
// `account-rail` plays it.

import { signInGap } from "../sign-in-gaps";

export const accountRail = signInGap("account-rail", "The account as a rail panel: a sign-in kept across putting it away, a second profile, sign out and a failed sign-in", {
  place: "rail",
  attempts: ["signs_in", "signs_in", { ends: "denied" }],
  storeDown: ["mele-work"],
});
