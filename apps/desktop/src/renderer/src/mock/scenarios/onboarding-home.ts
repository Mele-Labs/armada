// The home machine, first run: signed out, with the one Sign in with GitHub button in the title bar.
// Signing in shows the device code, then what got set up: the identity, the central store and the
// shared home, which opens already level with this machine's Kit. The walk `onboarding-home` plays it.

import type { SharedRow } from "@armada/components";

import type { Scenario } from "../moment";
import { WORK_ITEMS, withPlanUsage } from "../onboarding-items";
import { sharedHome } from "./shared-home";

const LEVEL = (row: SharedRow): SharedRow => ({ kind: row.kind, name: row.name, state: "same", ...(row.source === undefined ? {} : { source: row.source }), ...(row.says === undefined ? {} : { says: row.says }) });

function build(): Scenario {
  const home = sharedHome.draft?.sharedHome;
  return {
    ...sharedHome,
    name: "onboarding-home",
    says: "The home machine on first run: signed out, then the device code, then what signing in set up",
    behaves: withPlanUsage(sharedHome.behaves),
    draft: {
      ...(sharedHome.draft ?? {}),
      ...(home === undefined ? {} : { sharedHome: { ...home, rows: home.rows.map(LEVEL) } }),
      account: {
        signIn: { login: "nickmele", code: "WDJB-MJHT", repository: "armada-home" },
        add: { login: "mele-work", code: "KQXP-4N7D", items: WORK_ITEMS },
        machine: "studio-mac",
      },
    },
  };
}

export const onboardingHome: Scenario = build();
