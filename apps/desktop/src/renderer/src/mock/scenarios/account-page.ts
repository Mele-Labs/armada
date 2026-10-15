// The home machine with the account as a page in the work area, the title menu's one row opening it. A
// sign-in left mid-way waits where it was; the work login is added, signed out with the store down, and
// the next attempt ends turned down. The walk `account-page` plays it;

import { signInGap } from "../sign-in-gaps";

export const accountPage = signInGap("account-page", "The account as a page: a sign-in kept across leaving it, a second profile, sign out and a failed sign-in", {
  attempts: ["signs_in", "signs_in", { ends: "denied" }],
  storeDown: ["mele-work"],
});
