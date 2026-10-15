// The home machine already signed in to its own login and a work one added on purpose. Signing the work
// profile out leaves the list; signing the first out while the store is down leaves its row in the
// store. The walk `sign-out` plays it.

import { WORK_ITEMS } from "../onboarding-items";
import { signInGap } from "../sign-in-gaps";

export const signOut = signInGap("sign-out", "Two profiles signed in; the second signs out, then the first with the store down", {
  joined: [
    { login: "nickmele", chosen: false },
    { login: "mele-work", chosen: true },
  ],
  storeDown: ["nickmele"],
  add: { login: "mele-work", code: "KQXP-4N7D", items: WORK_ITEMS },
});
