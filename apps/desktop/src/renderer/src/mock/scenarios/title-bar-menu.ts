// The title row with one menu: the account, Sleep and Settings behind a single button, the plan's
// meters beside the search bar, and the repository picker in a panel at the top of the left column.
// Over two repositories, opening on All repositories, and signed out so the button carries its mark.
// The walk `title-bar-menu` plays it.

import type { Scenario } from "../moment";
import { PERSONAL_ITEMS, withPlanUsage } from "../onboarding-items";
import { prOwners } from "./pr-owners";

function build(): Scenario {
  return {
    ...prOwners,
    name: "title-bar-menu",
    says: "The title row with one menu, and the repository picker in the left column",
    behaves: withPlanUsage(prOwners.behaves),
    draft: {
      ...(prOwners.draft ?? {}),
      account: {
        signIn: { login: "nickmele", code: "WDJB-MJHT", repository: "armada-home" },
        add: { login: "mele-work", code: "KQXP-4N7D", items: PERSONAL_ITEMS },
      },
    },
  };
}

export const titleBarMenu: Scenario = build();
