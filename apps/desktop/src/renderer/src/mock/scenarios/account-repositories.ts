// Two profiles already signed in, each with the repositories bound to it: matched through the login or an
// organisation, one waiting where two profiles match, one where none does. The walk `account-repositories`
// plays it.

import { signInGap } from "../sign-in-gaps";

export const accountRepositories = signInGap("account-repositories", "Two profiles with the repositories bound to each, one waiting for a choice", {
  joined: [
    { login: "nickmele", chosen: false },
    { login: "mele-work", chosen: true },
  ],
  orgs: { nickmele: ["shared-org"], "mele-work": ["mele-labs", "shared-org"] },
  repositories: [
    "nickmele/recipe-scaler",
    "nickmele/armada-notes",
    "mele-labs/armada",
    "mele-labs/billing",
    "mele-labs/infra",
    "acme-co/vendor-lib",
    "shared-org/tools",
  ],
});
