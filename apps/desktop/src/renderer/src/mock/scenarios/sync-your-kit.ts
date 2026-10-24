// The home machine before any shared home exists, for the Account page's Sync your Kit frame: signed
// out, then signed in with the home still to be made by the first Sync on Kit. The walk `sync-your-kit`
// plays it;

import { signInGap } from "../sign-in-gaps";

export const syncYourKit = signInGap("sync-your-kit", "Account: the three steps to a second machine, the next one carrying its button", {
  noHome: true,
  orgs: { nickmele: ["mele-labs"] },
});
